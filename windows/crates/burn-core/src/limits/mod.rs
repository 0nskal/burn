//! Live rate limits of each provider. Blocking; call from a worker thread.

use crate::model::{LimitsError, ProviderId, ProviderLimits};
use crate::paths::Paths;

/// Cheap local check (credentials present). Never hits the network.
pub fn is_connected(provider: ProviderId, paths: &Paths) -> bool {
    let _ = (provider, paths);
    todo!()
}

pub fn fetch(provider: ProviderId, paths: &Paths) -> Result<ProviderLimits, LimitsError> {
    let _ = (provider, paths);
    todo!()
}
