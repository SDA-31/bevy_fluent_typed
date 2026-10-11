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

/// Default policy: load only scopes owned by inserted bindings and resource consumers.
///
/// Inserted `LocalizedText<S>` retains its scope until removed. On Bevy 0.19/0.20,
/// ordinary `Res<S>` systems retain it while their system state exists, including
/// aliases and derived parameters. Deferred startup consumers release their
/// demand after completion and deferred command application. Older backends use
/// localization scheduling helpers; one-time helpers release after invocation.
/// Optional `Localization::load`/`unload` calls add or remove idempotent manual
/// pins without evicting automatic consumers or overlapping pins. Unattached
/// messages and world inspection are passive. On Bevy 0.19/0.20,
/// `Option<PassiveRes<S>>` is passive; ordinary `Option<Res<S>>` is conservatively
/// a consumer because native requiredness is absent from scheduler metadata.
#[derive(Debug, Clone, Copy, Default)]
pub struct Lazy;

/// Load every module of the selected language.
#[derive(Debug, Clone, Copy, Default)]
pub struct Full;

/// Load only explicitly requested scopes; persistent requests give hybrid loading.
#[derive(Debug, Clone, Copy, Default)]
pub struct Manual;

impl sealed::Sealed for Full {}

impl sealed::Sealed for Manual {}

impl sealed::Sealed for Lazy {}

impl LoadingMode for Lazy {
	const FULL: bool = false;
	const AUTOMATIC: bool = true;
}

impl LoadingMode for Full {
	const FULL: bool = true;
	const AUTOMATIC: bool = false;
}

impl LoadingMode for Manual {
	const FULL: bool = false;
	const AUTOMATIC: bool = false;
}
