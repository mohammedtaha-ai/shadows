//! Reported turn usage values.
use super::HarnessEvent;

/// What the harness reported about a turn, from its last usage report
/// (§12.8: only the last is trusted — the first `size` is a guess). Nothing
/// it did not report is estimated.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TurnObservation {
    pub token_usage: Option<TurnUsage>,
    /// The harness session the turn ran in (§12.7 `native_session_id`).
    pub native_session_id: Option<String>,
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
                token_usage: None,
                native_session_id: None,
                observed_model: model.clone(),
                context_used: Some(*used),
                context_window: (*size > 0).then_some(*size),
            },
            _ => Self::default(),
        }
    }
}

/// Cumulative cost reported by a session, distinct from context usage.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UsageCost {
    pub amount: f64,
    pub currency: String,
}

/// Raw end-turn counters. They do not come from context notifications.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cached_read_tokens: u64,
    #[serde(default)]
    pub cached_write_tokens: u64,
    pub total_tokens: u64,
    #[serde(default)]
    pub cost: Option<UsageCost>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_observation_without_a_size_reports_no_window() {
        let u = HarnessEvent::Usage {
            used: 10,
            size: 0,
            model: None,
            rate_limit: None,
            cost: None,
        };
        let o = TurnObservation::from_usage(Some(&u));
        assert_eq!((o.context_used, o.context_window), (Some(10), None));
        assert_eq!(TurnObservation::from_usage(None).context_used, None);
    }
}
