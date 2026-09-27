//! Loading policy is selected in the plugin and controller types.
mod sealed {
	pub trait Sealed {}
}

/// A loading policy supported by this runtime. Implementations are sealed.
pub trait LoadingMode: sealed::Sealed + Send + Sync + 'static {
	/// Whether the root scope is always requested.
	#[doc(hidden)]
	const FULL: bool;
}

/// Load every module of the selected language.
#[derive(Debug, Clone, Copy, Default)]
pub struct Full;

/// Load only explicitly requested scopes; persistent requests give hybrid loading.
#[derive(Debug, Clone, Copy, Default)]
pub struct Lazy;

impl sealed::Sealed for Full {}

impl sealed::Sealed for Lazy {}

impl LoadingMode for Full {
	const FULL: bool = true;
}

impl LoadingMode for Lazy {
	const FULL: bool = false;
}
