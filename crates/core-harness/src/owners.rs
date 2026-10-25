use std::sync::Arc;

use core_agents::Agents;
use core_extensions::Extensions;
use core_person::PersonSettings;
use core_providers::Providers;

/// The configuration dependencies read by a harness opening.
#[derive(Clone)]
pub struct AylulOwners {
    pub providers: Arc<Providers>,
    pub agents: Arc<Agents>,
    pub extensions: Arc<Extensions>,
    pub person: PersonSettings,
}
