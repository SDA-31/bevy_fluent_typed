#![doc = include_str!("../docs/quickstart.md")]
#![doc = "\n```rust,ignore\n"]
#![doc = include_str!("../docs/typed_resources.rs")]
#![doc = "\n```\n"]
#![doc = include_str!("../docs/runtime.md")]
#![warn(missing_docs)]

#[cfg(not(any(feature = "bevy-0-17", feature = "bevy-0-18", feature = "bevy-0-19")))]
compile_error!("select one Bevy backend: bevy-0-17, bevy-0-18 or bevy-0-19 (default)");

#[cfg(any(
	all(feature = "bevy-0-17", feature = "bevy-0-18"),
	all(feature = "bevy-0-17", feature = "bevy-0-19"),
	all(feature = "bevy-0-18", feature = "bevy-0-19"),
))]
compile_error!(
	"Bevy backends are mutually exclusive; disable default features to select 0.17 or 0.18"
);

mod addresses;
mod assets;
mod bindings;
mod catalog;
mod compatibility;
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

/// The selected Bevy backend, also used by generated resource implementations.
#[cfg(feature = "bevy-0-19")]
#[doc(hidden)]
pub use bevy_0_19 as bevy;

#[cfg(all(feature = "bevy-0-18", not(feature = "bevy-0-19")))]
#[doc(hidden)]
pub use bevy_0_18 as bevy;

#[cfg(all(
	feature = "bevy-0-17",
	not(any(feature = "bevy-0-18", feature = "bevy-0-19"))
))]
#[doc(hidden)]
pub use bevy_0_17 as bevy;

#[cfg(test)]
mod tests;
