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
#[path = "tests/contract.rs"]
mod contract_tests;

#[cfg(test)]
#[path = "tests/native.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/custom.rs"]
mod custom_tests;

#[cfg(test)]
#[path = "tests/cache_policy.rs"]
mod cache_policy_tests;

#[cfg(test)]
#[path = "tests/query.rs"]
mod query_tests;

#[cfg(test)]
#[path = "tests/initialization.rs"]
mod initialization_tests;

#[cfg(all(test, feature = "manifest"))]
#[path = "tests/lifecycle.rs"]
mod lifecycle_tests;

#[cfg(test)]
#[path = "tests/consumer.rs"]
mod consumer_tests;

#[cfg(test)]
#[path = "tests/spans.rs"]
mod spans_tests;

#[cfg(all(test, any(feature = "bevy-0-19", feature = "bevy-0-20")))]
#[path = "tests/template.rs"]
mod template_tests;

#[cfg(test)]
#[path = "tests/basics.rs"]
mod basics_tests;
