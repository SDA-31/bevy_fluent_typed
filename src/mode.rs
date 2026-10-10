//! Loading policy is selected in the plugin and controller types.
mod sealed {
	pub trait Sealed {}
}

/// A loading policy supported by this runtime. Implementations are sealed.
pub trait LoadingMode: sealed::Sealed + Send + Sync + 'static {
	/// Whether the root scope is always requested.
	#[doc(hidden)]
	const FULL: bool;

	/// Whether inserted bindings and localized systems establish demand.
	#[doc(hidden)]
	const AUTOMATIC: bool;
}

/// Default policy: load only scopes owned by inserted bindings and localized systems.
///
/// Inserted `LocalizedText<S>` retains its scope until removed. Required direct
/// `Res<S>` parameters registered with localization helpers retain it while their
/// system state exists. One-time helpers release their demand after invocation.
/// Optional `Localization::load`/`unload` calls add or remove idempotent manual
/// pins without evicting automatic consumers or overlapping pins. Unattached
/// messages, optional resources and world inspection are passive.
#[derive(Debug, Clone, Copy, Default)]
pub struct Auto;

/// Load every module of the selected language.
#[derive(Debug, Clone, Copy, Default)]
pub struct Full;

/// Load only explicitly requested scopes; persistent requests give hybrid loading.
#[derive(Debug, Clone, Copy, Default)]
pub struct Lazy;

impl sealed::Sealed for Full {}

impl sealed::Sealed for Lazy {}

impl sealed::Sealed for Auto {}

impl LoadingMode for Auto {
	const FULL: bool = false;
	const AUTOMATIC: bool = true;
}

impl LoadingMode for Full {
	const FULL: bool = true;
	const AUTOMATIC: bool = false;
}

impl LoadingMode for Lazy {
	const FULL: bool = false;
	const AUTOMATIC: bool = false;
}
