//! Session-scoped cumulative cost accounting.
use super::events::UsageCost;

#[derive(Default)]
pub(crate) struct SessionCost {
    pub(crate) previous: Option<UsageCost>,
    pub(crate) current: Option<UsageCost>,
    pub(crate) active: bool,
}

pub(crate) fn cost_delta(
    previous: Option<&UsageCost>,
    current: Option<&UsageCost>,
) -> Option<UsageCost> {
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
