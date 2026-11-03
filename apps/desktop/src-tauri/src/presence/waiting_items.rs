//! The panel's count and navigation share the same projection of open requests.
use super::core_client::{WaitingGroup, WaitingPermission};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Target<'a> {
    Permission { index: usize },
    Group { id: &'a str },
}

pub(super) fn items<'a>(
    groups: &'a [WaitingGroup],
    permissions: &'a [WaitingPermission],
) -> Vec<(&'a str, Target<'a>)> {
    let mut by_thread = BTreeMap::new();
    let mut by_group = BTreeMap::new();
    for group in groups {
        let id = group.id.as_str();
        for member in &group.members {
            by_thread.insert(member.thread_id.as_str(), id);
        }
        if let Some(since) = group.needs_you_since.as_deref() {
            by_group.insert(id, utc_sort_key(since));
        }
    }
    let mut items = Vec::new();
    for (index, request) in permissions.iter().enumerate() {
        let time = utc_sort_key(&request.asked_at);
        if let Some(&id) = by_thread.get(request.thread_id.as_str()) {
            let earliest = by_group.entry(id).or_insert(time);
            *earliest = (*earliest).min(time);
        } else {
            items.push((time, Target::Permission { index }));
        }
    }
    items.extend(
        by_group
            .into_iter()
            .map(|(id, time)| (time, Target::Group { id })),
    );
    items
}

fn utc_sort_key(timestamp: &str) -> &str {
    // Core's UTC timestamps can have different fractional precision.
    timestamp.strip_suffix('Z').unwrap_or(timestamp)
}
