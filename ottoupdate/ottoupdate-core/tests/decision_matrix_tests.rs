use chrono::{Duration, Utc};
use ottoupdate_core::decision::matrix::{
    DecisionInput, DecisionMatrix, PolicyDecision, UpdateDecision,
};

#[derive(Clone)]
struct DecisionInputBuilder {
    input: DecisionInput,
}

impl Default for DecisionInputBuilder {
    fn default() -> Self {
        Self {
            input: DecisionInput {
                policy_decision: PolicyDecision::Approve,
                device_safe: true,
                battery_percent: Some(100),
                disk_free_mb: Some(10_000),
                update_size_mb: Some(20),
                in_maintenance_window: true,
                deferred_count: 0,
                max_deferred_days: 7,
                breaking_changes: false,
                requires_reboot: false,
                network_metered: false,
                staged_percentage: None,
                device_in_canary: true,
                manifest_revoked: false,
                approval_granted: true,
                now: Utc::now(),
                next_maintenance_window_start: Some(Utc::now() + Duration::hours(2)),
            },
        }
    }
}

impl DecisionInputBuilder {
    fn build(self) -> DecisionInput {
        self.input
    }

    fn policy_decision(mut self, decision: PolicyDecision) -> Self {
        self.input.policy_decision = decision;
        self
    }

    fn device_safe(mut self, value: bool) -> Self {
        self.input.device_safe = value;
        self
    }

    fn battery_percent(mut self, value: Option<u8>) -> Self {
        self.input.battery_percent = value;
        self
    }

    fn disk_free_mb(mut self, value: Option<u64>) -> Self {
        self.input.disk_free_mb = value;
        self
    }

    fn update_size_mb(mut self, value: Option<u64>) -> Self {
        self.input.update_size_mb = value;
        self
    }

    fn in_maintenance_window(mut self, value: bool) -> Self {
        self.input.in_maintenance_window = value;
        self
    }

    fn deferred_count(mut self, value: u32) -> Self {
        self.input.deferred_count = value;
        self
    }

    fn max_deferred_days(mut self, value: u32) -> Self {
        self.input.max_deferred_days = value;
        self
    }

    fn breaking_changes(mut self, value: bool) -> Self {
        self.input.breaking_changes = value;
        self
    }

    fn requires_reboot(mut self, value: bool) -> Self {
        self.input.requires_reboot = value;
        self
    }

    fn network_metered(mut self, value: bool) -> Self {
        self.input.network_metered = value;
        self
    }

    fn staged_percentage(mut self, value: Option<u8>) -> Self {
        self.input.staged_percentage = value;
        self
    }

    fn device_in_canary(mut self, value: bool) -> Self {
        self.input.device_in_canary = value;
        self
    }

    fn manifest_revoked(mut self, value: bool) -> Self {
        self.input.manifest_revoked = value;
        self
    }

    fn approval_granted(mut self, value: bool) -> Self {
        self.input.approval_granted = value;
        self
    }
}

fn assert_kind(decision: UpdateDecision, expected: &str) {
    match (decision, expected) {
        (UpdateDecision::Approve, "approve") => {}
        (UpdateDecision::Defer { .. }, "defer") => {}
        (UpdateDecision::Block { .. }, "block") => {}
        (UpdateDecision::RequireApproval { .. }, "require_approval") => {}
        (actual, _) => panic!("unexpected decision: {actual:?}"),
    }
}

#[test]
// Scenario 1: revoked manifest must always block.
fn rule_01_manifest_revoked_blocks() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default().manifest_revoked(true).build();
    assert_kind(matrix.evaluate(&input), "block");
}

#[test]
// Scenario 2: non-revoked manifest allows subsequent rule evaluation.
fn rule_02_manifest_not_revoked_does_not_block() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default().build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 3: policy block should return blocked decision.
fn rule_03_policy_block_wins() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .policy_decision(PolicyDecision::Block {
            reason: "blocked_by_policy".to_string(),
        })
        .build();
    assert_kind(matrix.evaluate(&input), "block");
}

#[test]
// Scenario 4: non-block policy should continue to later rules.
fn rule_04_non_block_policy_skips_rule_2() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .policy_decision(PolicyDecision::Approve)
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 5: unsafe device with battery 19 should defer.
fn rule_05_low_battery_positive() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .device_safe(false)
        .battery_percent(Some(19))
        .build();
    assert_kind(matrix.evaluate(&input), "defer");
}

#[test]
// Scenario 6: battery boundary at 20 should not trigger low-battery deferral.
fn rule_06_low_battery_boundary_20() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .device_safe(false)
        .battery_percent(Some(20))
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 7: unsafe device with 511MB free should defer for low disk.
fn rule_07_low_disk_positive() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .device_safe(false)
        .battery_percent(Some(90))
        .disk_free_mb(Some(511))
        .build();
    assert_kind(matrix.evaluate(&input), "defer");
}

#[test]
// Scenario 8: disk boundary at 512MB should not trigger low-disk deferral.
fn rule_08_low_disk_boundary_512() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .device_safe(false)
        .battery_percent(Some(90))
        .disk_free_mb(Some(512))
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 9: metered network and large payload should defer until unmetered.
fn rule_09_metered_large_download_defers() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .network_metered(true)
        .update_size_mb(Some(101))
        .build();
    assert_kind(matrix.evaluate(&input), "defer");
}

#[test]
// Scenario 10: metered network but small payload should continue.
fn rule_10_metered_small_download_skips() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .network_metered(true)
        .update_size_mb(Some(100))
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 11: unmetered network and large payload should continue.
fn rule_11_unmetered_large_download_skips() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .network_metered(false)
        .update_size_mb(Some(200))
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 12: reboot-required outside maintenance window should defer.
fn rule_12_reboot_outside_window_defers() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .requires_reboot(true)
        .in_maintenance_window(false)
        .build();
    assert_kind(matrix.evaluate(&input), "defer");
}

#[test]
// Scenario 13: reboot-required inside maintenance window should continue.
fn rule_13_reboot_inside_window_skips() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .requires_reboot(true)
        .in_maintenance_window(true)
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 14: policy require-approval should return approval requirement.
fn rule_14_policy_require_approval_positive() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .policy_decision(PolicyDecision::RequireApproval {
            group: "ops".to_string(),
        })
        .build();
    assert_kind(matrix.evaluate(&input), "require_approval");
}

#[test]
// Scenario 15: approve policy should not trigger require-approval rule.
fn rule_15_policy_require_approval_negative() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .policy_decision(PolicyDecision::Approve)
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 16: staged rollout set and device outside canary should defer.
fn rule_16_staged_rollout_positive() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .staged_percentage(Some(20))
        .device_in_canary(false)
        .build();
    assert_kind(matrix.evaluate(&input), "defer");
}

#[test]
// Scenario 17: staged rollout set but device in canary should continue.
fn rule_17_staged_rollout_negative_in_canary() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .staged_percentage(Some(20))
        .device_in_canary(true)
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 18: no staged percentage should skip staged rollout rule.
fn rule_18_staged_rollout_negative_unset() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .staged_percentage(None)
        .device_in_canary(false)
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 19: deferred count equal to max should force approve.
fn rule_19_force_approve_equal_boundary() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .deferred_count(7)
        .max_deferred_days(7)
        .policy_decision(PolicyDecision::Defer {
            until: Utc::now() + Duration::hours(2),
            reason: "policy_defer".to_string(),
        })
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 20: deferred count above max should force approve.
fn rule_20_force_approve_above_boundary() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .deferred_count(8)
        .max_deferred_days(7)
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 21: deferred count below max should not force approve.
fn rule_21_force_approve_negative() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .deferred_count(6)
        .max_deferred_days(7)
        .policy_decision(PolicyDecision::Defer {
            until: Utc::now() + Duration::hours(2),
            reason: "policy_defer".to_string(),
        })
        .build();
    assert_kind(matrix.evaluate(&input), "defer");
}

#[test]
// Scenario 22: policy defer should map to update defer when no earlier match.
fn rule_22_policy_defer_positive() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .policy_decision(PolicyDecision::Defer {
            until: Utc::now() + Duration::hours(3),
            reason: "defer_by_policy".to_string(),
        })
        .build();
    assert_kind(matrix.evaluate(&input), "defer");
}

#[test]
// Scenario 23: non-defer policy should skip policy defer rule.
fn rule_23_policy_defer_negative() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .policy_decision(PolicyDecision::Approve)
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 24: breaking changes without approval should require approval.
fn rule_24_breaking_change_requires_approval_positive() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .breaking_changes(true)
        .approval_granted(false)
        .build();
    assert_kind(matrix.evaluate(&input), "require_approval");
}

#[test]
// Scenario 25: breaking changes with approval should skip rule.
fn rule_25_breaking_change_requires_approval_negative() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .breaking_changes(true)
        .approval_granted(true)
        .build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 26: default approve applies when no rule matches.
fn rule_26_default_approve() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default().build();
    assert_kind(matrix.evaluate(&input), "approve");
}

#[test]
// Scenario 27: explain should stop on first matched rule.
fn rule_27_explain_stops_on_first_match() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default().manifest_revoked(true).build();
    let steps = matrix.explain(&input);
    assert!(steps.first().map(|s| s.matched).unwrap_or(false));
    assert_eq!(steps.len(), 1);
}

#[test]
// Scenario 28: policy block should take precedence over low battery.
fn rule_28_precedence_policy_block_before_low_battery() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .policy_decision(PolicyDecision::Block {
            reason: "policy".to_string(),
        })
        .device_safe(false)
        .battery_percent(Some(10))
        .build();
    assert_kind(matrix.evaluate(&input), "block");
}

#[test]
// Scenario 29: low battery should take precedence over low disk.
fn rule_29_precedence_low_battery_before_low_disk() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .device_safe(false)
        .battery_percent(Some(5))
        .disk_free_mb(Some(100))
        .build();

    let decision = matrix.evaluate(&input);
    match decision {
        UpdateDecision::Defer { reason, .. } => assert_eq!(reason, "low_battery"),
        _ => panic!("expected low_battery deferral"),
    }
}

#[test]
// Scenario 30: force-approve should override policy-defer at defer ceiling.
fn rule_30_force_approve_overrides_policy_defer() {
    let matrix = DecisionMatrix;
    let input = DecisionInputBuilder::default()
        .deferred_count(10)
        .max_deferred_days(10)
        .policy_decision(PolicyDecision::Defer {
            until: Utc::now() + Duration::hours(9),
            reason: "policy_defer".to_string(),
        })
        .build();

    assert_kind(matrix.evaluate(&input), "approve");
}
