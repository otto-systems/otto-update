use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Datelike, Duration, NaiveTime, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Approve,
    Defer { until: DateTime<Utc>, reason: String },
    Block { reason: String },
    RequireApproval { group: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceState {
    pub device_id: String,
    pub product: String,
    pub channel: String,
    pub platform: String,
    pub architecture: String,
    pub management_group: Option<String>,
    pub tags: Vec<String>,
    pub installed_version: Option<String>,
    pub deferred_count: u32,
    pub last_deferred_at: Option<DateTime<Utc>>,
    pub observed: BTreeMap<String, Value>,
}

impl DeviceState {
    fn field_value(&self, field: &str) -> Value {
        match field {
            "product" => Value::String(self.product.clone()),
            "channel" => Value::String(self.channel.clone()),
            "platform" => Value::String(self.platform.clone()),
            "architecture" => Value::String(self.architecture.clone()),
            "management_group" => self
                .management_group
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
            "deferred_count" => Value::Number(self.deferred_count.into()),
            "installed_version" => self
                .installed_version
                .clone()
                .map(Value::String)
                .unwrap_or(Value::Null),
            _ => self.observed.get(field).cloned().unwrap_or(Value::Null),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TargetCriteria {
    pub products: Vec<String>,
    pub channels: Vec<String>,
    pub platforms: Vec<String>,
    pub architectures: Vec<String>,
    pub management_groups: Vec<String>,
    pub tags_any: Vec<String>,
    pub tags_all: Vec<String>,
    pub min_installed_version: Option<String>,
    pub max_installed_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    All,
    Any,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuleCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    NotIn,
    Contains,
    Exists,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleActionType {
    Approve,
    Defer,
    Block,
    RequireApproval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleAction {
    pub action_type: RuleActionType,
    pub reason: String,
    pub defer_seconds: Option<i64>,
    pub approval_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyRule {
    pub rule_id: String,
    pub priority: i32,
    pub enabled: bool,
    pub match_mode: MatchMode,
    pub conditions: Vec<RuleCondition>,
    pub action: RuleAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaintenanceWindow {
    pub days_of_week: Vec<String>,
    pub start_time: String,
    pub end_time: String,
    pub timezone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlobalReleasePolicy {
    pub policy_id: String,
    pub priority: i32,
    pub target_criteria: TargetCriteria,
    pub rules: Vec<PolicyRule>,
    pub maintenance_windows: Option<Vec<MaintenanceWindow>>,
    pub max_deferred_days: u32,
    pub approval_group: Option<String>,
}

pub struct PolicyEngine {
    policies: Vec<GlobalReleasePolicy>,
}

impl PolicyEngine {
    pub fn new(policies: Vec<GlobalReleasePolicy>) -> Self {
        Self { policies }
    }

    pub async fn evaluate(&self, state: &DeviceState) -> PolicyDecision {
        self.evaluate_at(state, Utc::now()).await
    }

    pub async fn evaluate_at(&self, state: &DeviceState, now: DateTime<Utc>) -> PolicyDecision {
        debug!(device_id = %state.device_id, "policy evaluation started");

        let mut matching: Vec<&GlobalReleasePolicy> = self
            .policies
            .iter()
            .filter(|policy| policy_matches(policy, state))
            .collect();

        matching.sort_by(|a, b| b.priority.cmp(&a.priority));
        debug!(matches = matching.len(), "policies matched target criteria");

        for policy in matching {
            debug!(policy_id = %policy.policy_id, priority = policy.priority, "evaluating policy");

            if let Some(decision) = self.evaluate_policy_rules(policy, state, now).await {
                debug!(policy_id = %policy.policy_id, ?decision, "policy produced decision");
                return decision;
            }
        }

        debug!("no policy rule matched; using default approve action");
        PolicyDecision::Approve
    }

    async fn evaluate_policy_rules(
        &self,
        policy: &GlobalReleasePolicy,
        state: &DeviceState,
        now: DateTime<Utc>,
    ) -> Option<PolicyDecision> {
        let mut rules: Vec<&PolicyRule> = policy.rules.iter().filter(|rule| rule.enabled).collect();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in rules {
            debug!(
                policy_id = %policy.policy_id,
                rule_id = %rule.rule_id,
                rule_priority = rule.priority,
                "evaluating rule"
            );

            if rule_matches(rule, state).await {
                debug!(policy_id = %policy.policy_id, rule_id = %rule.rule_id, "first matching rule selected");
                let base_decision = action_to_decision(&rule.action, now);
                let bounded = self.apply_deferral_escalation(policy, state, base_decision, now);
                return Some(self.apply_maintenance_windows(policy, bounded, now));
            }
        }

        let default_decision = self.apply_deferral_escalation(policy, state, PolicyDecision::Approve, now);
        Some(self.apply_maintenance_windows(policy, default_decision, now))
    }

    fn apply_deferral_escalation(
        &self,
        policy: &GlobalReleasePolicy,
        state: &DeviceState,
        decision: PolicyDecision,
        now: DateTime<Utc>,
    ) -> PolicyDecision {
        if state.deferred_count < policy.max_deferred_days {
            return decision;
        }

        debug!(
            deferred_count = state.deferred_count,
            max_deferred_days = policy.max_deferred_days,
            policy_id = %policy.policy_id,
            "defer limit reached; escalating decision"
        );

        match policy.approval_group.as_deref() {
            Some(group) => PolicyDecision::RequireApproval {
                group: group.to_string(),
            },
            None => {
                debug!(at = %now.to_rfc3339(), "forcing approval after defer ceiling");
                PolicyDecision::Approve
            }
        }
    }

    fn apply_maintenance_windows(
        &self,
        policy: &GlobalReleasePolicy,
        decision: PolicyDecision,
        now: DateTime<Utc>,
    ) -> PolicyDecision {
        match decision {
            PolicyDecision::Approve => {
                if is_in_maintenance_window(policy.maintenance_windows.as_ref(), now) {
                    PolicyDecision::Approve
                } else {
                    let until = next_maintenance_start(policy.maintenance_windows.as_ref(), now)
                        .unwrap_or_else(|| now + Duration::hours(6));

                    debug!(
                        policy_id = %policy.policy_id,
                        until = %until.to_rfc3339(),
                        "outside maintenance window; converting approval to defer"
                    );

                    PolicyDecision::Defer {
                        until,
                        reason: "outside_maintenance_window".to_string(),
                    }
                }
            }
            other => other,
        }
    }
}

#[allow(async_fn_in_trait)]
pub trait ConditionEvaluator: Send + Sync {
    async fn evaluate(&self, state: &DeviceState) -> bool;
}

struct EqConditionEvaluator {
    field: String,
    expected: Value,
}

#[allow(async_fn_in_trait)]
impl ConditionEvaluator for EqConditionEvaluator {
    async fn evaluate(&self, state: &DeviceState) -> bool {
        state.field_value(&self.field) == self.expected
    }
}

struct NeqConditionEvaluator {
    field: String,
    expected: Value,
}

#[allow(async_fn_in_trait)]
impl ConditionEvaluator for NeqConditionEvaluator {
    async fn evaluate(&self, state: &DeviceState) -> bool {
        state.field_value(&self.field) != self.expected
    }
}

struct CompareConditionEvaluator {
    field: String,
    expected: Value,
    op: ConditionOperator,
}

#[allow(async_fn_in_trait)]
impl ConditionEvaluator for CompareConditionEvaluator {
    async fn evaluate(&self, state: &DeviceState) -> bool {
        let left = state.field_value(&self.field);

        let left_num = left.as_f64();
        let right_num = self.expected.as_f64();
        if let (Some(a), Some(b)) = (left_num, right_num) {
            return match self.op {
                ConditionOperator::Gt => a > b,
                ConditionOperator::Gte => a >= b,
                ConditionOperator::Lt => a < b,
                ConditionOperator::Lte => a <= b,
                _ => false,
            };
        }

        let left_str = left.as_str();
        let right_str = self.expected.as_str();
        if let (Some(a), Some(b)) = (left_str, right_str) {
            return match self.op {
                ConditionOperator::Gt => a > b,
                ConditionOperator::Gte => a >= b,
                ConditionOperator::Lt => a < b,
                ConditionOperator::Lte => a <= b,
                _ => false,
            };
        }

        false
    }
}

struct InConditionEvaluator {
    field: String,
    values: Vec<Value>,
    inverted: bool,
}

#[allow(async_fn_in_trait)]
impl ConditionEvaluator for InConditionEvaluator {
    async fn evaluate(&self, state: &DeviceState) -> bool {
        let value = state.field_value(&self.field);
        let contains = self.values.iter().any(|candidate| candidate == &value);
        if self.inverted {
            !contains
        } else {
            contains
        }
    }
}

struct ContainsConditionEvaluator {
    field: String,
    expected: Value,
}

#[allow(async_fn_in_trait)]
impl ConditionEvaluator for ContainsConditionEvaluator {
    async fn evaluate(&self, state: &DeviceState) -> bool {
        let value = state.field_value(&self.field);
        match value {
            Value::Array(items) => items.iter().any(|v| v == &self.expected),
            Value::String(s) => self
                .expected
                .as_str()
                .map(|needle| s.contains(needle))
                .unwrap_or(false),
            _ => false,
        }
    }
}

struct ExistsConditionEvaluator {
    field: String,
}

#[allow(async_fn_in_trait)]
impl ConditionEvaluator for ExistsConditionEvaluator {
    async fn evaluate(&self, state: &DeviceState) -> bool {
        !state.field_value(&self.field).is_null()
    }
}

async fn rule_matches(rule: &PolicyRule, state: &DeviceState) -> bool {
    let mut evaluations = Vec::with_capacity(rule.conditions.len());

    for condition in &rule.conditions {
        let evaluator = build_evaluator(condition);
        let passed = evaluator.evaluate(state).await;

        debug!(
            rule_id = %rule.rule_id,
            field = %condition.field,
            operator = ?condition.operator,
            passed,
            "condition evaluation"
        );

        evaluations.push(passed);
    }

    match rule.match_mode {
        MatchMode::All => evaluations.into_iter().all(std::convert::identity),
        MatchMode::Any => evaluations.into_iter().any(std::convert::identity),
    }
}

fn build_evaluator(condition: &RuleCondition) -> Box<dyn ConditionEvaluator> {
    match condition.operator {
        ConditionOperator::Eq => Box::new(EqConditionEvaluator {
            field: condition.field.clone(),
            expected: condition.value.clone(),
        }),
        ConditionOperator::Neq => Box::new(NeqConditionEvaluator {
            field: condition.field.clone(),
            expected: condition.value.clone(),
        }),
        ConditionOperator::Gt
        | ConditionOperator::Gte
        | ConditionOperator::Lt
        | ConditionOperator::Lte => Box::new(CompareConditionEvaluator {
            field: condition.field.clone(),
            expected: condition.value.clone(),
            op: condition.operator.clone(),
        }),
        ConditionOperator::In => Box::new(InConditionEvaluator {
            field: condition.field.clone(),
            values: condition.value.as_array().cloned().unwrap_or_default(),
            inverted: false,
        }),
        ConditionOperator::NotIn => Box::new(InConditionEvaluator {
            field: condition.field.clone(),
            values: condition.value.as_array().cloned().unwrap_or_default(),
            inverted: true,
        }),
        ConditionOperator::Contains => Box::new(ContainsConditionEvaluator {
            field: condition.field.clone(),
            expected: condition.value.clone(),
        }),
        ConditionOperator::Exists => Box::new(ExistsConditionEvaluator {
            field: condition.field.clone(),
        }),
    }
}

fn action_to_decision(action: &RuleAction, now: DateTime<Utc>) -> PolicyDecision {
    match action.action_type {
        RuleActionType::Approve => PolicyDecision::Approve,
        RuleActionType::Defer => PolicyDecision::Defer {
            until: now + Duration::seconds(action.defer_seconds.unwrap_or(3600)),
            reason: action.reason.clone(),
        },
        RuleActionType::Block => PolicyDecision::Block {
            reason: action.reason.clone(),
        },
        RuleActionType::RequireApproval => PolicyDecision::RequireApproval {
            group: action
                .approval_group
                .clone()
                .unwrap_or_else(|| "default".to_string()),
        },
    }
}

fn policy_matches(policy: &GlobalReleasePolicy, state: &DeviceState) -> bool {
    let tc = &policy.target_criteria;

    if !tc.products.is_empty() && !tc.products.iter().any(|p| p == &state.product) {
        return false;
    }
    if !tc.channels.is_empty() && !tc.channels.iter().any(|c| c == &state.channel) {
        return false;
    }
    if !tc.platforms.is_empty() && !tc.platforms.iter().any(|p| p == &state.platform) {
        return false;
    }
    if !tc.architectures.is_empty() && !tc.architectures.iter().any(|a| a == &state.architecture) {
        return false;
    }

    if !tc.management_groups.is_empty() {
        match &state.management_group {
            Some(group) if tc.management_groups.iter().any(|g| g == group) => {}
            _ => return false,
        }
    }

    let tag_set: HashSet<&str> = state.tags.iter().map(String::as_str).collect();
    if !tc.tags_any.is_empty() && !tc.tags_any.iter().any(|tag| tag_set.contains(tag.as_str())) {
        return false;
    }
    if !tc.tags_all.is_empty() && !tc.tags_all.iter().all(|tag| tag_set.contains(tag.as_str())) {
        return false;
    }

    if !version_in_range(
        state.installed_version.as_deref(),
        tc.min_installed_version.as_deref(),
        tc.max_installed_version.as_deref(),
    ) {
        return false;
    }

    true
}

fn version_in_range(current: Option<&str>, min: Option<&str>, max: Option<&str>) -> bool {
    let Some(current) = current else {
        return true;
    };

    let Some(current_v) = parse_semver_prefix(current) else {
        return true;
    };

    if let Some(min_v) = min.and_then(parse_semver_prefix) {
        if current_v < min_v {
            return false;
        }
    }

    if let Some(max_v) = max.and_then(parse_semver_prefix) {
        if current_v > max_v {
            return false;
        }
    }

    true
}

fn parse_semver_prefix(v: &str) -> Option<(u32, u32, u32)> {
    let core = v.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse::<u32>().ok()?;
    let minor = parts.next()?.parse::<u32>().ok()?;
    let patch = parts.next()?.parse::<u32>().ok()?;
    Some((major, minor, patch))
}

fn is_in_maintenance_window(
    windows: Option<&Vec<MaintenanceWindow>>,
    now: DateTime<Utc>,
) -> bool {
    let Some(windows) = windows else {
        return true;
    };

    windows.iter().any(|w| window_contains(w, now))
}

fn next_maintenance_start(
    windows: Option<&Vec<MaintenanceWindow>>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let windows = windows?;
    let mut nearest: Option<DateTime<Utc>> = None;

    for day_offset in 0..=7 {
        let date = now.date_naive() + Duration::days(day_offset);
        let weekday = date.weekday();

        for window in windows {
            let weekdays = parse_weekdays(&window.days_of_week);
            if !weekdays.contains(&weekday) {
                continue;
            }

            let Some(start) = parse_hhmm(&window.start_time) else {
                continue;
            };

            let candidate_naive = date.and_time(start);
            let candidate = Utc.from_utc_datetime(&candidate_naive);

            if candidate <= now {
                continue;
            }

            nearest = match nearest {
                Some(current) if current <= candidate => Some(current),
                _ => Some(candidate),
            };
        }
    }

    nearest
}

fn window_contains(window: &MaintenanceWindow, now: DateTime<Utc>) -> bool {
    let weekdays = parse_weekdays(&window.days_of_week);
    if !weekdays.contains(&now.weekday()) {
        return false;
    }

    let Some(start) = parse_hhmm(&window.start_time) else {
        return false;
    };
    let Some(end) = parse_hhmm(&window.end_time) else {
        return false;
    };

    let current = now.time();
    if start <= end {
        current >= start && current <= end
    } else {
        current >= start || current <= end
    }
}

fn parse_weekdays(days: &[String]) -> HashSet<Weekday> {
    days.iter()
        .filter_map(|d| match d.as_str() {
            "mon" => Some(Weekday::Mon),
            "tue" => Some(Weekday::Tue),
            "wed" => Some(Weekday::Wed),
            "thu" => Some(Weekday::Thu),
            "fri" => Some(Weekday::Fri),
            "sat" => Some(Weekday::Sat),
            "sun" => Some(Weekday::Sun),
            _ => None,
        })
        .collect()
}

fn parse_hhmm(value: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value, "%H:%M").ok()
}
