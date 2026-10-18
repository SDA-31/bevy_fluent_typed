#![cfg_attr(feature = "runtime", doc = include_str!("../docs/quickstart.md"))]
#![cfg_attr(feature = "runtime", doc = include_str!("../docs/runtime.md"))]
#![cfg_attr(feature = "runtime", doc = include_str!("../docs/asset-sources.md"))]
#![cfg_attr(feature = "runtime", doc = include_str!("../docs/formatting.md"))]
#![cfg_attr(feature = "runtime", doc = include_str!("../docs/bsn.md"))]
#![cfg_attr(feature = "runtime", doc = include_str!("../docs/bindings.md"))]
#![cfg_attr(feature = "build", doc = include_str!("../docs/build.md"))]
#![warn(missing_docs)]

#[cfg(all(
	not(any(
		feature = "bevy-0-16",
		feature = "bevy-0-17",
		feature = "bevy-0-18",
		feature = "bevy-0-19",
		feature = "bevy-0-20"
	)),
	any(
		not(feature = "build"),
		feature = "runtime",
		feature = "codegen",
		feature = "watch"
	)
))]
compile_error!(
	"select one Bevy backend: bevy-0-16, bevy-0-17, bevy-0-18, bevy-0-19 or bevy-0-20 (default)"
);

#[cfg(any(
	all(
		feature = "bevy-0-16",
		any(feature = "bevy-0-17", feature = "bevy-0-18", feature = "bevy-0-19")
	),
	all(feature = "bevy-0-17", feature = "bevy-0-18"),
	all(feature = "bevy-0-17", feature = "bevy-0-19"),
	all(feature = "bevy-0-18", feature = "bevy-0-19"),
	all(
		feature = "bevy-0-20",
		any(
			feature = "bevy-0-16",
			feature = "bevy-0-17",
			feature = "bevy-0-18",
			feature = "bevy-0-19"
		)
	),
))]
compile_error!(
	"Bevy backends are mutually exclusive; disable default features to select another backend"
);

#[cfg(all(feature = "runtime", feature = "manifest"))]
mod addresses;
#[cfg(all(feature = "runtime", feature = "manifest"))]
mod assets;
#[cfg(feature = "runtime")]
mod bindings;
#[cfg(feature = "runtime")]
mod bytes;
#[cfg(feature = "runtime")]
mod cache_policy;
#[cfg(feature = "runtime")]
mod catalog;
#[cfg(feature = "runtime")]
mod compatibility;
#[cfg(feature = "runtime")]
mod demand;
#[cfg(feature = "build")]
mod generation;
#[cfg(feature = "runtime")]
mod leases;
#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17", feature = "bevy-0-18"))]
mod legacy_systems;
#[cfg(feature = "runtime")]
mod loading;
#[cfg(feature = "runtime")]
mod message;
#[cfg(feature = "runtime")]
mod mode;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
mod passive;
#[cfg(feature = "runtime")]
mod plugin;
#[cfg(feature = "runtime")]
mod preparation;
#[cfg(feature = "runtime")]
mod progress;
#[cfg(feature = "runtime")]
mod registration;
#[cfg(feature = "runtime")]
mod resources;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
mod scheduling;
#[cfg(feature = "runtime")]
mod scope;
#[cfg(feature = "runtime")]
mod state;
#[cfg(feature = "runtime")]
mod status;
#[cfg(feature = "runtime")]
mod target_bindings;

#[cfg(all(feature = "codegen", feature = "runtime"))]
mod macros;

/// Explicit build-script generation and configuration, without an engine backend.
#[cfg(feature = "build")]
pub use generation::{build, from_cargo, generate};

/// Generator settings for explicit build frontends.
#[cfg(feature = "build")]
pub use fluent_typed_codegen::{
	BuildError, IoOperation, ModuleMismatch, NameError, NameOwner, SchemaError, Settings,
	SyntaxNode, UpstreamShapeError,
};

/// Parser used by generated validation contracts.
#[cfg(feature = "codegen")]
#[doc(hidden)]
pub use fluent_syntax;

#[cfg(feature = "runtime")]
pub use cache_policy::CachePolicy;
#[cfg(feature = "runtime")]
pub use catalog::{FluentCatalog, FluentScope, Module};
#[cfg(any(feature = "manifest", feature = "build"))]
#[doc(hidden)]
pub use fluent_typed_codegen as __fluent_codegen;
#[cfg(feature = "manifest")]
pub use fluent_typed_codegen::{LocalizationManifest, ManifestError};

/// Typed configuration failures shared by manifest parsing and host generation.
#[cfg(any(feature = "manifest", feature = "build"))]
pub use fluent_typed_codegen::{CatalogConfig, ConfigError, ConfigField, FieldError, PathError};
#[cfg(feature = "runtime")]
pub use leases::ModuleLease;
#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17", feature = "bevy-0-18"))]
pub use legacy_systems::{IntoLocalizedSystems, LocalizationAppExt, localized};
#[cfg(feature = "runtime")]
#[allow(
	deprecated,
	reason = "Re-export the compatibility alias for existing consumers"
)]
pub use message::Message;
#[cfg(feature = "runtime")]
pub use message::{CatalogUpdate, CatalogUpdateReader, Msg, ReloadCatalogs};
#[cfg(feature = "runtime")]
pub use mode::{Full, Lazy, LoadingMode, Manual};
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
pub use passive::PassiveCatalog;
#[cfg(feature = "runtime")]
pub use plugin::{LocalizationAccessibilitySet, LocalizationPlugin, LocalizationSystems};
#[cfg(feature = "runtime")]
pub use preparation::{CommitLocaleError, PreparationStatus};
#[cfg(all(feature = "runtime", feature = "diagnostics"))]
pub use progress::ModuleDiagnostic;
#[cfg(feature = "runtime")]
pub use progress::{LoadingProgress, LocalizationProgress, LocalizationProgressPlugin};
#[cfg(feature = "runtime")]
pub use registration::LocalizationRegistrationError;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
pub use scheduling::{LocalizationStartupPlugin, StartupStrategy};
#[cfg(feature = "runtime")]
pub use scope::{ModuleStore, ScopeRegistration};
#[cfg(feature = "runtime")]
pub use state::{LocaleView, LocaleViewMut, Localization};
#[cfg(feature = "runtime")]
pub use status::{ModuleError, ModuleStatus};
#[cfg(feature = "runtime")]
pub use target_bindings::{
	LocalizationTarget, LocalizationUpdate, Localized, TargetItem, TargetReadItem,
	TargetUpdatePhase, UpdateResult,
};

/// Compatible upstream runtime used by generated accessors; no separate dependency needed.
#[cfg(feature = "runtime")]
pub use fluent_typed;

/// The selected Bevy backend, also used by generated resource implementations.
#[cfg(feature = "bevy-0-20")]
#[doc(hidden)]
pub use bevy_0_20 as bevy;

#[cfg(all(feature = "bevy-0-19", not(feature = "bevy-0-20")))]
#[doc(hidden)]
pub use bevy_0_19 as bevy;

#[cfg(all(
	feature = "bevy-0-18",
	not(any(feature = "bevy-0-19", feature = "bevy-0-20"))
))]
#[doc(hidden)]
pub use bevy_0_18 as bevy;

#[cfg(all(
	feature = "bevy-0-17",
	not(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20"))
))]
#[doc(hidden)]
pub use bevy_0_17 as bevy;

#[cfg(all(
	feature = "bevy-0-16",
	not(any(
		feature = "bevy-0-17",
		feature = "bevy-0-18",
		feature = "bevy-0-19",
		feature = "bevy-0-20"
	))
))]
#[doc(hidden)]
pub use bevy_0_16 as bevy;

#[cfg(all(test, feature = "runtime"))]
mod tests;
