use std::sync::Arc;

use crate::agents::Agents;
use crate::extensions::Extensions;
use crate::person::PersonSettings;
use crate::providers::Providers;

/// The configuration dependencies read by a harness opening.
#[derive(Clone)]
pub(crate) struct AylulOwners {
    pub(crate) providers: Arc<Providers>,
    pub(crate) agents: Arc<Agents>,
    pub(crate) extensions: Arc<Extensions>,
    pub(crate) person: PersonSettings,
}
