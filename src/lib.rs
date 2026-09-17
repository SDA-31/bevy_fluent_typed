#![cfg_attr(feature = "runtime", doc = include_str!("../docs/quickstart.md"))]
#![cfg_attr(feature = "runtime", doc = "\n```rust,ignore\n")]
#![cfg_attr(feature = "runtime", doc = include_str!("../docs/typed_resources.rs"))]
#![cfg_attr(feature = "runtime", doc = "\n```\n")]
#![cfg_attr(feature = "runtime", doc = include_str!("../docs/runtime.md"))]
#![cfg_attr(feature = "build", doc = include_str!("../docs/build.md"))]
#![warn(missing_docs)]

#[cfg(all(
	not(any(feature = "bevy-0-17", feature = "bevy-0-18", feature = "bevy-0-19")),
	any(
		not(feature = "build"),
		feature = "runtime",
		feature = "codegen",
		feature = "watch"
	)
))]
compile_error!("select one Bevy backend: bevy-0-17, bevy-0-18 or bevy-0-19 (default)");

#[cfg(any(
	all(feature = "bevy-0-17", feature = "bevy-0-18"),
	all(feature = "bevy-0-17", feature = "bevy-0-19"),
	all(feature = "bevy-0-18", feature = "bevy-0-19"),
))]
compile_error!(
	"Bevy backends are mutually exclusive; disable default features to select 0.17 or 0.18"
);

#[cfg(feature = "runtime")]
mod addresses;
#[cfg(feature = "runtime")]
mod assets;
#[cfg(feature = "runtime")]
mod bindings;
#[cfg(feature = "runtime")]
mod catalog;
#[cfg(feature = "runtime")]
mod compatibility;
#[cfg(feature = "runtime")]
mod message;
#[cfg(feature = "runtime")]
mod plugin;
#[cfg(feature = "runtime")]
mod resources;
#[cfg(feature = "runtime")]
mod state;

#[cfg(all(feature = "codegen", feature = "runtime"))]
mod macros;

#[doc(hidden)]
#[cfg(feature = "codegen")]
pub use bevy_fluent_codegen_bridge as __codegen;

/// Explicit build-script generation and configuration, without an engine backend.
#[cfg(feature = "build")]
pub use bevy_fluent_codegen_bridge::{Settings, build, from_cargo, generate};

#[cfg(feature = "runtime")]
pub use catalog::{CatalogDescriptor, FluentCatalog, Module, ModuleSource};
#[cfg(feature = "runtime")]
pub use message::{CatalogUpdate, LocalizedText, Message, ReloadCatalogs};
#[cfg(feature = "runtime")]
pub use plugin::{LocalizationPlugin, LocalizationSystems};
#[cfg(feature = "runtime")]
pub use state::Localization;

/// Compatible upstream runtime used by generated accessors; no separate dependency needed.
#[cfg(feature = "runtime")]
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

#[cfg(all(test, feature = "runtime"))]
mod tests;
