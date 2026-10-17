//! Extensible entity-owned bindings and application component adapters.
mod component;
mod context;
mod deferred;
mod engine;
mod parallel;
mod refresh;
mod state;
mod targets;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
mod template;

pub use component::{
	LocalizationTarget, Localized, TargetItem, TargetReadItem, TargetUpdatePhase, UpdateResult,
};
pub use context::LocalizationUpdate;

#[cfg(test)]
#[path = "target_bindings/tests/contract.rs"]
mod contract_tests;

#[cfg(test)]
#[path = "target_bindings/tests/native.rs"]
mod tests;

#[cfg(test)]
#[path = "target_bindings/tests/custom.rs"]
mod custom_tests;

#[cfg(test)]
#[path = "target_bindings/tests/cache_policy.rs"]
mod cache_policy_tests;

#[cfg(test)]
#[path = "target_bindings/tests/query.rs"]
mod query_tests;

#[cfg(test)]
#[path = "target_bindings/tests/initialization.rs"]
mod initialization_tests;
