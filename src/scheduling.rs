//! Native system discovery and startup lifecycle coordination.
mod access;
mod configuration;
mod lifecycle;
mod readiness;
mod startup;
mod system;

pub(crate) use configuration::StartupConfiguration;
pub use configuration::StartupStrategy;
pub(crate) use lifecycle::register;
pub(crate) use readiness::CatalogReadiness;
