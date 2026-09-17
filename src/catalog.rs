//! Application-supplied generated API and its modular source catalogs.
use crate::bevy::prelude::World;
use std::path::PathBuf;

/// Validated source layout, relative to the loaded definition asset's directory.
///
/// A provider interprets its own definition format. The runtime validates these
/// asset addresses, watches declared modules and publishes checked snapshots.
pub struct CatalogDescriptor {
	/// Directory containing locale subdirectories; `.` means the definition's parent.
	/// Must be a nonempty UTF-8 relative path without parent/root/prefix components,
	/// backslashes, `#` or `:`. Modules inherit the definition's named Bevy asset source.
	pub modules_directory: PathBuf,
}

/// One source module relative to a language directory, with its original embedded text.
#[derive(Clone, Copy, Debug)]
pub struct Module {
	/// Relative UTF-8 path below a locale directory, e.g. `ui/menu.ftl`.
	/// Must name a file, with no parent/root/prefix components, backslashes, `#` or `:`.
	pub path: &'static str,
	/// Original source used to recognize unchanged external modules.
	/// Must correspond to [`FluentCatalog::embedded`]; schema validation is provider-owned.
	pub embedded: &'static str,
}

/// One external Fluent module, identified by path rather than input order.
#[derive(Clone, Copy, Debug)]
pub struct ModuleSource<'a> {
	/// Relative path below the language directory.
	pub path: &'a str,
	/// Unmodified Fluent source for this module.
	pub source: &'a str,
}

/// Runtime contract for an application's typed catalog.
///
/// Providers supply descriptors and checked snapshots; generation and definition
/// formats are not part of this runtime contract. The optional companion
/// `bevy_fluent_codegen_bridge` generates an implementation for directory catalogs.
///
/// Locale identifiers must be unique nonempty directory names. The locale list
/// must include the default, and every locale must provide the same module/API
/// schema. Embedded catalogs are trusted build-time-validated fallbacks.
pub trait FluentCatalog: Send + Sync + Sized + 'static {
	/// Stable locale identifier; `AsRef<str>` must return its directory name.
	type Locale: Copy + Eq + Send + Sync + AsRef<str> + 'static;

	/// All compiled locales, in a stable, nonempty order, with no duplicates.
	fn locales() -> &'static [Self::Locale];

	/// Startup language, which must belong to [`Self::locales`].
	fn default_locale() -> Self::Locale;

	/// Interpret and validate an opaque definition asset before loading its modules.
	///
	/// # Errors
	/// Reject incompatible definitions without changing any published language.
	fn descriptor(definition: &[u8]) -> Result<CatalogDescriptor, String>;

	/// Load a build-validated embedded catalog without filesystem access.
	fn embedded(locale: Self::Locale) -> Self;

	/// Every source module for a locale; paths and schemas must match across locales.
	/// Paths must be unique and iteration order stable across calls, so unchanged
	/// ordered sources can retain their snapshot and module-resource change-detection state.
	fn modules(locale: Self::Locale) -> Vec<Module>;

	/// Parse external modules without discarding their namespaces.
	///
	/// # Errors
	/// Return a diagnostic when any source cannot supply this catalog's typed API.
	/// Providers must validate complete module inventory, keys, references and typed
	/// arguments here. The runtime never publishes a partially checked candidate.
	fn parse(locale: Self::Locale, sources: &[ModuleSource<'_>]) -> Result<Self, String>;

	/// Publish immutable module resources from this complete catalog snapshot.
	///
	/// Called with exclusive World access at initialization and when the active
	/// snapshot changes. Generated implementations share parsed data through `Arc`.
	/// Publish the entire resource tree in this call; ordinary scheduled systems
	/// cannot observe intermediate inserts.
	/// Custom providers without module resources may keep this default.
	fn publish_resources(&self, _world: &mut World) {}
}
