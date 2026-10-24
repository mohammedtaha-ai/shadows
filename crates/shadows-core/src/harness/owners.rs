use std::sync::Arc;

use crate::agents::Agents;
use crate::extensions::Extensions;
use core_person::PersonSettings;
use core_providers::Providers;

/// The configuration dependencies read by a harness opening.
#[derive(Clone)]
pub(crate) struct AylulOwners {
    pub(crate) providers: Arc<Providers>,
    pub(crate) agents: Arc<Agents>,
    pub(crate) extensions: Arc<Extensions>,
    pub(crate) person: PersonSettings,
}
