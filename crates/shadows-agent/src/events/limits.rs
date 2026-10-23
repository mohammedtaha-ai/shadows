//! Account limit reports.
use serde_json::Value;

/// One account limit window as the harness reported it: `utilization` from 0
/// to 1, `resets_at` in Unix seconds.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct LimitWindow {
    pub utilization: f64,
    pub resets_at: i64,
}

/// The account's limits as last reported by the harness (spec §12.8). A
/// window the harness did not report is `None`, never estimated.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct AccountLimits {
    #[schema(required)]
    pub five_hour: Option<LimitWindow>,
    #[schema(required)]
    pub seven_day: Option<LimitWindow>,
    /// When the daemon received the report (RFC 3339).
    pub observed_at: String,
}

impl AccountLimits {
    /// Reads Claude's rate-limit report (`_meta["_claude/rateLimit"]` on a
    /// `usage_update`): `unifiedWindows.five_hour` and `.seven_day`, each
    /// `{ utilization, resetsAt }`. `None` when neither window is present, so a
    /// report without figures is no limits rather than zero.
    pub fn from_report(rate_limit: &Value, observed_at: &str) -> Option<AccountLimits> {
        let windows = rate_limit.get("unifiedWindows")?;
        let window = |name: &str| {
            let w = windows.get(name)?;
            Some(LimitWindow {
                utilization: w.get("utilization")?.as_f64()?,
                resets_at: w.get("resetsAt")?.as_i64()?,
            })
        };
        let (five_hour, seven_day) = (window("five_hour"), window("seven_day"));
        if five_hour.is_none() && seven_day.is_none() {
            return None;
        }
        Some(AccountLimits {
            five_hour,
            seven_day,
            observed_at: observed_at.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn limits_read_both_windows_with_reset_times() {
        let v = json!({ "unifiedWindows": {
            "five_hour": { "utilization": 0.75, "resetsAt": 1790212200 },
            "seven_day": { "utilization": 0.91, "resetsAt": 1790542800 } } });
        let l = AccountLimits::from_report(&v, "2026-09-24T10:00:00Z").unwrap();
        assert_eq!(l.five_hour.unwrap().resets_at, 1790212200);
        assert!((l.seven_day.unwrap().utilization - 0.91).abs() < 1e-9);
        assert_eq!(l.observed_at, "2026-09-24T10:00:00Z");
    }
    #[test]
    fn a_report_without_windows_is_no_limits_rather_than_zero() {
        assert!(AccountLimits::from_report(&json!({ "status": "allowed" }), "t").is_none());
        assert!(AccountLimits::from_report(&json!({ "unifiedWindows": {} }), "t").is_none());
    }
}
