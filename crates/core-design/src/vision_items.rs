//! Pure vision item edits with server-assigned identities.
use super::model::{VisionContent, VisionCounters, VisionField, VisionItem, WorkspaceOp};
use core_threads::ThreadId;

impl VisionField {
    pub const ALL: [Self; 5] = [
        Self::Purpose,
        Self::Users,
        Self::Goals,
        Self::Boundaries,
        Self::TechnicalDirection,
    ];

    pub fn prefix(self) -> &'static str {
        match self {
            Self::Purpose => "PU",
            Self::Users => "U",
            Self::Goals => "G",
            Self::Boundaries => "B",
            Self::TechnicalDirection => "TD",
        }
    }

    pub fn of_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|field| {
            id.strip_prefix(field.prefix()).is_some_and(|rest| {
                !rest.is_empty()
                    && !rest.starts_with('0')
                    && rest.bytes().all(|c| c.is_ascii_digit())
                    && rest.parse::<u32>().is_ok()
            })
        })
    }
}

impl VisionContent {
    pub fn items(&self, field: VisionField) -> &[VisionItem] {
        match field {
            VisionField::Purpose => &self.purpose,
            VisionField::Users => &self.users,
            VisionField::Goals => &self.goals,
            VisionField::Boundaries => &self.boundaries,
            VisionField::TechnicalDirection => &self.technical_direction,
        }
    }

    fn items_mut(&mut self, field: VisionField) -> &mut Vec<VisionItem> {
        match field {
            VisionField::Purpose => &mut self.purpose,
            VisionField::Users => &mut self.users,
            VisionField::Goals => &mut self.goals,
            VisionField::Boundaries => &mut self.boundaries,
            VisionField::TechnicalDirection => &mut self.technical_direction,
        }
    }

    pub fn item(&self, id: &str) -> Option<&VisionItem> {
        self.items(VisionField::of_id(id)?)
            .iter()
            .find(|i| i.id == id)
    }
}

impl VisionItems {
    pub fn next_id(field: VisionField, counters: &mut VisionCounters) -> Result<String, String> {
        let slot = match field {
            VisionField::Purpose => &mut counters.purpose,
            VisionField::Users => &mut counters.users,
            VisionField::Goals => &mut counters.goals,
            VisionField::Boundaries => &mut counters.boundaries,
            VisionField::TechnicalDirection => &mut counters.technical_direction,
        };
        *slot = slot.checked_add(1).ok_or("vision identity exhausted")?;
        Ok(format!("{}{slot}", field.prefix()))
    }
}

impl VisionItems {
    pub fn apply(
        content: &mut VisionContent,
        op: &WorkspaceOp,
        id_for_add: Option<String>,
        from: Option<&ThreadId>,
    ) -> Result<(), String> {
        match op {
            WorkspaceOp::VisionItemAdd { field, text } => {
                if text.trim().is_empty() {
                    return Err("a vision item needs text".into());
                }
                let id = id_for_add.ok_or("a vision addition needs its assigned id")?;
                content.items_mut(*field).push(VisionItem {
                    id,
                    text: text.clone(),
                    approved_from: from.cloned(),
                    revision: 1,
                });
            }
            WorkspaceOp::VisionItemPut { id, text } => {
                if text.trim().is_empty() {
                    return Err("a vision item needs text".into());
                }
                let field =
                    VisionField::of_id(id).ok_or_else(|| format!("{id} is not in the vision"))?;
                let item = content
                    .items_mut(field)
                    .iter_mut()
                    .find(|i| &i.id == id)
                    .ok_or_else(|| format!("{id} is not in the vision"))?;
                item.revision = item
                    .revision
                    .checked_add(1)
                    .ok_or("vision item revision exhausted")?;
                item.text = text.clone();
                item.approved_from = from.cloned();
            }
            WorkspaceOp::VisionItemRemove { id } => {
                let field =
                    VisionField::of_id(id).ok_or_else(|| format!("{id} is not in the vision"))?;
                let items = content.items_mut(field);
                let index = items
                    .iter()
                    .position(|i| &i.id == id)
                    .ok_or_else(|| format!("{id} is not in the vision"))?;
                items.remove(index);
            }
            _ => unreachable!("only vision ops reach the item editor"),
        }
        Ok(())
    }
}

pub struct VisionItems;
