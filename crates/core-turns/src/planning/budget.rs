//! Character-based planning context budgets.
pub(super) const INDEX_TOKENS: i64 = 1_500;
pub(super) const DETAIL_TOKENS: i64 = 8_000;

pub(super) struct Budget;

impl Budget {
    pub(super) fn estimate(text: &str) -> i64 {
        text.chars().count().div_ceil(4) as i64
    }
}
