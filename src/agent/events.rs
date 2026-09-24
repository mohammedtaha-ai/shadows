use serde_json::Value;

/// Transient reports from one ACP connection.
#[derive(Debug, Clone)]
pub enum HarnessEvent {
    Chunk {
        message_id: Option<String>,
        text: String,
    },
    ToolCall {
        id: String,
        title: Option<String>,
        status: Option<String>,
    },
    PermissionRefused {
        title: String,
    },
    Usage {
        used: u64,
        size: u64,
        model: Option<String>,
        rate_limit: Option<Value>,
    },
    Options(Value),
    /// The prompt has answered and the turn's entries are durable. Sent by the
    /// Planner's watcher, not the connection: ACP ends a turn with the prompt's
    /// response, which no notification carries.
    TurnEnd {
        subtype: &'static str,
        stop_reason: Option<String>,
    },
}

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

/// Reads Claude's rate-limit report (`_meta["_claude/rateLimit"]` on a
/// `usage_update`): `unifiedWindows.five_hour` and `.seven_day`, each
/// `{ utilization, resetsAt }`. `None` when neither window is present, so a
/// report without figures is no limits rather than zero.
pub fn limits_from(rate_limit: &Value, observed_at: &str) -> Option<AccountLimits> {
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

/// What the harness reported about a turn, from its last usage report
/// (§12.8: only the last is trusted — the first `size` is a guess). Nothing
/// it did not report is estimated.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurnObservation {
    pub observed_model: Option<String>,
    pub context_used: Option<u64>,
    pub context_window: Option<u64>,
}

impl TurnObservation {
    /// From the turn's last `Usage`, or nothing when it reported none. A
    /// `size` of 0 is no usable window.
    pub fn from_usage(last: Option<&HarnessEvent>) -> Self {
        match last {
            Some(HarnessEvent::Usage {
                used, size, model, ..
            }) => Self {
                observed_model: model.clone(),
                context_used: Some(*used),
                context_window: (*size > 0).then_some(*size),
            },
            _ => Self::default(),
        }
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
        let l = limits_from(&v, "2026-09-24T10:00:00Z").unwrap();
        assert_eq!(l.five_hour.unwrap().resets_at, 1790212200);
        assert!((l.seven_day.unwrap().utilization - 0.91).abs() < 1e-9);
        assert_eq!(l.observed_at, "2026-09-24T10:00:00Z");
    }

    #[test]
    fn a_report_without_windows_is_no_limits_rather_than_zero() {
        assert!(limits_from(&json!({ "status": "allowed" }), "t").is_none());
        assert!(limits_from(&json!({ "unifiedWindows": {} }), "t").is_none());
    }

    #[test]
    fn an_observation_without_a_size_reports_no_window() {
        let u = HarnessEvent::Usage {
            used: 10,
            size: 0,
            model: None,
            rate_limit: None,
        };
        let o = TurnObservation::from_usage(Some(&u));
        assert_eq!((o.context_used, o.context_window), (Some(10), None));
        assert_eq!(TurnObservation::from_usage(None).context_used, None);
    }
}
