mod systems;

pub use systems::{IntoLocalizedSystems, LocalizationAppExt, localized};

pub(crate) use systems::{CatalogReadiness, ParameterTypes};

#[cfg(all(
	test,
	any(feature = "bevy-0-16", feature = "bevy-0-17", feature = "bevy-0-18")
))]
#[path = "tests/systems.rs"]
mod systems_tests;
