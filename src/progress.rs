//! Native loading counters and optional per-module inspection.
#[cfg(feature = "diagnostics")]
mod diagnostics;
mod publication;
mod registration;
mod resource;
mod snapshot;
mod tracking;

#[cfg(feature = "diagnostics")]
pub use diagnostics::ModuleDiagnostic;
#[cfg(feature = "diagnostics")]
pub(crate) use diagnostics::inspect as inspect_modules;
#[cfg(test)]
pub(crate) use publication::synchronize as publish;
pub(crate) use registration::{Registration, register, request};
pub use resource::LocalizationProgress;
pub use snapshot::LoadingProgress;
pub(crate) use snapshot::inspect;
pub(crate) use tracking::{ObservedSet, StoreVersion};
