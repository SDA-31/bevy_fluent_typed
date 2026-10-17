//! Native system discovery and startup lifecycle coordination.
mod access;
mod configuration;
mod lifecycle;
mod plugin;
mod readiness;
mod startup;
mod system;

pub use configuration::StartupStrategy;
pub use plugin::LocalizationStartupPlugin;
pub(crate) use readiness::CatalogReadiness;

#[cfg(all(test, any(feature = "bevy-0-19", feature = "bevy-0-20")))]
#[path = "tests/resources.rs"]
mod resources_tests;

#[cfg(all(test, feature = "manifest"))]
#[path = "tests/scheduling.rs"]
mod scheduling_tests;

#[cfg(all(test, any(feature = "bevy-0-19", feature = "bevy-0-20")))]
#[path = "tests/startup.rs"]
mod startup_tests;

#[cfg(all(
	test,
	feature = "manifest",
	any(feature = "bevy-0-19", feature = "bevy-0-20")
))]
#[path = "tests/waiting.rs"]
mod waiting_tests;
