#![doc = include_str!("../docs/quickstart.md")]
#![doc = "\n```rust,ignore\n"]
#![doc = include_str!("../docs/typed_resources.rs")]
#![doc = "\n```\n"]
#![doc = include_str!("../docs/runtime.md")]
#![warn(missing_docs)]
mod addresses;
mod assets;
mod bindings;
mod catalog;
mod message;
mod plugin;
mod resources;
mod state;

#[cfg(feature = "codegen")]
mod macros;

#[doc(hidden)]
#[cfg(feature = "codegen")]
pub use bevy_fluent_codegen_bridge as __codegen;

pub use catalog::{CatalogDescriptor, FluentCatalog, Module, ModuleSource};
pub use message::{CatalogUpdate, LocalizedText, Message, ReloadCatalogs};
pub use plugin::{LocalizationPlugin, LocalizationSystems};
pub use state::Localization;

/// Compatible upstream runtime used by generated accessors; no separate dependency needed.
pub use fluent_typed;

/// Bevy paths used by generated resource implementations.
#[doc(hidden)]
pub use bevy;

#[cfg(test)]
mod tests;
