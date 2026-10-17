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
