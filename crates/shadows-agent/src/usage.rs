//! Session-scoped cumulative cost accounting.
use crate::events::UsageCost;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct SessionCost {
    previous: Option<UsageCost>,
    current: Option<UsageCost>,
    active: bool,
}

#[derive(Clone, Default)]
pub(crate) struct Costs {
    sessions: Arc<Mutex<HashMap<String, SessionCost>>>,
}

impl Costs {
    pub(crate) fn open(&self, session: &str, fresh: bool) {
        self.sessions.lock().expect("cost ledger").insert(
            session.into(),
            SessionCost {
                previous: fresh.then(|| UsageCost {
                    amount: 0.0,
                    currency: "USD".into(),
                }),
                ..Default::default()
            },
        );
    }

    pub(crate) fn record(&self, session: &str, cost: Option<UsageCost>) {
        let mut sessions = self.sessions.lock().expect("cost ledger");
        if let Some(state) = sessions.get_mut(session)
            && state.active
        {
            state.current = cost;
        }
    }

    pub(crate) fn begin<'a>(&'a self, session: &'a str) -> Result<ActiveCost<'a>, String> {
        let mut sessions = self.sessions.lock().expect("cost ledger");
        let state = sessions.entry(session.into()).or_default();
        if state.active {
            return Err("session prompt already active".into());
        }
        state.current = None;
        state.active = true;
        Ok(ActiveCost {
            costs: self,
            session,
        })
    }
}

pub(crate) struct ActiveCost<'a> {
    costs: &'a Costs,
    session: &'a str,
}

impl ActiveCost<'_> {
    pub(crate) fn finish(self) -> Option<UsageCost> {
        let mut sessions = self.costs.sessions.lock().expect("cost ledger");
        let state = sessions.entry(self.session.into()).or_default();
        let delta = cost_delta(state.previous.as_ref(), state.current.as_ref());
        state.previous = state.current.take();
        delta
    }
}

impl Drop for ActiveCost<'_> {
    fn drop(&mut self) {
        let mut sessions = self.costs.sessions.lock().expect("cost ledger");
        if let Some(state) = sessions.get_mut(self.session) {
            state.active = false;
        }
    }
}

fn cost_delta(previous: Option<&UsageCost>, current: Option<&UsageCost>) -> Option<UsageCost> {
    let (previous, current) = (previous?, current?);
    (previous.currency == "USD"
        && current.currency == "USD"
        && previous.amount.is_finite()
        && current.amount.is_finite()
        && previous.amount >= 0.0
        && current.amount >= previous.amount)
        .then(|| UsageCost {
            amount: current.amount - previous.amount,
            currency: "USD".into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usd(amount: f64) -> UsageCost {
        UsageCost {
            amount,
            currency: "USD".into(),
        }
    }

    #[test]
    fn dropping_a_prompt_releases_the_shared_session_lease() {
        let costs = Costs::default();
        costs.open("session", true);
        let active = costs.begin("session").unwrap();
        let clone = costs.clone();
        assert!(clone.begin("session").is_err());
        costs.record("session", Some(usd(0.2)));
        drop(active);
        costs.record("session", Some(usd(0.4)));
        let next = clone.begin("session").unwrap();
        assert!(next.finish().is_none());
    }

    #[test]
    fn session_cost_baselines_are_independent() {
        let costs = Costs::default();
        costs.open("new", true);
        costs.open("resumed", false);
        let fresh = costs.begin("new").unwrap();
        let resumed = costs.begin("resumed").unwrap();
        costs.record("new", Some(usd(0.2)));
        costs.record("resumed", Some(usd(3.0)));
        assert_eq!(fresh.finish(), Some(usd(0.2)));
        assert!(resumed.finish().is_none());
        let next = costs.begin("resumed").unwrap();
        costs.record("resumed", Some(usd(3.5)));
        assert_eq!(next.finish(), Some(usd(0.5)));
    }
    #[test]
    fn session_costs_are_deltas() {
        let usd = |amount| UsageCost {
            amount,
            currency: "USD".into(),
        };
        let first = cost_delta(Some(&usd(0.0)), Some(&usd(0.2))).unwrap();
        let second = cost_delta(Some(&usd(0.2)), Some(&usd(0.5))).unwrap();
        assert!((first.amount - 0.2).abs() < 1e-12);
        assert!((second.amount - 0.3).abs() < 1e-12);
        for amount in [f64::NAN, f64::INFINITY, -1.0, 0.1] {
            assert!(cost_delta(Some(&usd(0.2)), Some(&usd(amount))).is_none());
        }
        assert!(cost_delta(None, Some(&usd(0.5))).is_none());
        assert!(cost_delta(Some(&usd(0.5)), None).is_none());
        assert!(
            cost_delta(
                Some(&usd(0.0)),
                Some(&UsageCost {
                    amount: 0.2,
                    currency: "EUR".into()
                })
            )
            .is_none()
        );
    }
}
