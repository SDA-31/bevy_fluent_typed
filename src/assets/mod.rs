mod loader;
mod preparation;

pub(crate) use loader::{ModuleAsset, ModuleLoader};
pub(crate) use preparation::{
	AssetAttempt, PreparationAttempts, PreparationGuard, PreparedModuleAsset, PreparedModuleLoader,
};

#[cfg(all(test, feature = "manifest"))]
#[path = "tests/loader.rs"]
mod loader_tests;
