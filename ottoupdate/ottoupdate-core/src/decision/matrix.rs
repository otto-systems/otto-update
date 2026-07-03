use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyDecision {
    Approve,
    Defer { until: DateTime<Utc>, reason: String },
    Block { reason: String },
    RequireApproval { group: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateDecision {
    Approve,
    Defer {
        until: Option<DateTime<Utc>>,
        reason: String,
    },
    RequireApproval {
        group: String,
    },
    Block {
        reason: String,
    },
}

#[derive(Debug, Clone)]
pub struct DecisionInput {
    pub policy_decision: PolicyDecision,
    pub device_safe: bool,
    pub battery_percent: Option<u8>,
    pub disk_free_mb: Option<u64>,
    pub update_size_mb: Option<u64>,
    pub in_maintenance_window: bool,
    pub deferred_count: u32,
    pub max_deferred_days: u32,
    pub breaking_changes: bool,
    pub requires_reboot: bool,
    pub network_metered: bool,
    pub staged_percentage: Option<u8>,
    pub device_in_canary: bool,
    pub manifest_revoked: bool,
    pub approval_granted: bool,
    pub now: DateTime<Utc>,
    pub next_maintenance_window_start: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct DecisionStep {
    pub rule_name: String,
    pub matched: bool,
    pub decision: Option<UpdateDecision>,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct DecisionMatrix;

impl DecisionMatrix {
    pub fn evaluate(&self, input: &DecisionInput) -> UpdateDecision {
        if let Some(decision) = self.rule_manifest_revoked(input) {
            return decision;
        }
        if let Some(decision) = self.rule_policy_block(input) {
            return decision;
        }
        if let Some(decision) = self.rule_low_battery(input) {
            return decision;
        }
        if let Some(decision) = self.rule_low_disk(input) {
            return decision;
        }
        if let Some(decision) = self.rule_metered_large_download(input) {
            return decision;
        }
        if let Some(decision) = self.rule_reboot_outside_window(input) {
            return decision;
        }
        if let Some(decision) = self.rule_policy_require_approval(input) {
            return decision;
        }
        if let Some(decision) = self.rule_staged_rollout(input) {
            return decision;
        }
        if let Some(decision) = self.rule_force_approve_after_max_defers(input) {
            return decision;
        }
        if let Some(decision) = self.rule_policy_defer(input) {
            return decision;
        }
        if let Some(decision) = self.rule_breaking_change_requires_approval(input) {
            return decision;
        }

        self.rule_default(input)
    }

    pub fn explain(&self, input: &DecisionInput) -> Vec<DecisionStep> {
        let mut steps = Vec::new();

        let rules: [(&str, RuleFn); 12] = [
            ("rule_manifest_revoked", Self::rule_manifest_revoked),
            ("rule_policy_block", Self::rule_policy_block),
            ("rule_low_battery", Self::rule_low_battery),
            ("rule_low_disk", Self::rule_low_disk),
            ("rule_metered_large_download", Self::rule_metered_large_download),
            ("rule_reboot_outside_window", Self::rule_reboot_outside_window),
            ("rule_policy_require_approval", Self::rule_policy_require_approval),
            ("rule_staged_rollout", Self::rule_staged_rollout),
            (
                "rule_force_approve_after_max_defers",
                Self::rule_force_approve_after_max_defers,
            ),
            ("rule_policy_defer", Self::rule_policy_defer),
            (
                "rule_breaking_change_requires_approval",
                Self::rule_breaking_change_requires_approval,
            ),
            ("rule_default", Self::rule_default_wrapped),
        ];

        for (name, rule) in rules {
            let outcome = rule(self, input);
            let matched = outcome.is_some();
            steps.push(DecisionStep {
                rule_name: name.to_string(),
                matched,
                decision: outcome.clone(),
                detail: if matched {
                    "matched".to_string()
                } else {
                    "skipped".to_string()
                },
            });

            if matched {
                break;
            }
        }

        steps
    }

    fn rule_manifest_revoked(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        if input.manifest_revoked {
            return Some(UpdateDecision::Block {
                reason: "manifest_revoked".to_string(),
            });
        }
        None
    }

    fn rule_policy_block(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        match &input.policy_decision {
            PolicyDecision::Block { reason } => Some(UpdateDecision::Block {
                reason: reason.clone(),
            }),
            _ => None,
        }
    }

    fn rule_low_battery(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        if input.device_safe {
            return None;
        }

        if input.battery_percent.unwrap_or(100) < 20 {
            return Some(UpdateDecision::Defer {
                until: Some(input.now + Duration::hours(4)),
                reason: "low_battery".to_string(),
            });
        }

        None
    }

    fn rule_low_disk(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        if input.device_safe {
            return None;
        }

        if input.disk_free_mb.unwrap_or(u64::MAX) < 512 {
            return Some(UpdateDecision::Defer {
                until: Some(input.now + Duration::hours(4)),
                reason: "low_disk".to_string(),
            });
        }

        None
    }

    fn rule_metered_large_download(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        if input.network_metered && input.update_size_mb.unwrap_or(0) > 100 {
            return Some(UpdateDecision::Defer {
                until: None,
                reason: "until_unmetered".to_string(),
            });
        }

        None
    }

    fn rule_reboot_outside_window(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        if !input.in_maintenance_window && input.requires_reboot {
            return Some(UpdateDecision::Defer {
                until: input.next_maintenance_window_start,
                reason: "until_window".to_string(),
            });
        }

        None
    }

    fn rule_policy_require_approval(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        match &input.policy_decision {
            PolicyDecision::RequireApproval { group } => Some(UpdateDecision::RequireApproval {
                group: group.clone(),
            }),
            _ => None,
        }
    }

    fn rule_staged_rollout(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        if input.staged_percentage.is_some() && !input.device_in_canary {
            return Some(UpdateDecision::Defer {
                until: Some(input.now + Duration::hours(24)),
                reason: "staged_rollout".to_string(),
            });
        }

        None
    }

    fn rule_force_approve_after_max_defers(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        if input.deferred_count >= input.max_deferred_days {
            return Some(UpdateDecision::Approve);
        }

        None
    }

    fn rule_policy_defer(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        match &input.policy_decision {
            PolicyDecision::Defer { until, reason } => Some(UpdateDecision::Defer {
                until: Some(*until),
                reason: reason.clone(),
            }),
            _ => None,
        }
    }

    fn rule_breaking_change_requires_approval(
        &self,
        input: &DecisionInput,
    ) -> Option<UpdateDecision> {
        if input.breaking_changes && !input.approval_granted {
            return Some(UpdateDecision::RequireApproval {
                group: "default".to_string(),
            });
        }

        None
    }

    fn rule_default(&self, _input: &DecisionInput) -> UpdateDecision {
        UpdateDecision::Approve
    }

    fn rule_default_wrapped(&self, input: &DecisionInput) -> Option<UpdateDecision> {
        Some(self.rule_default(input))
    }
}

type RuleFn = fn(&DecisionMatrix, &DecisionInput) -> Option<UpdateDecision>;
