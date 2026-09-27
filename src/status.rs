//! Selected-language availability and the latest loading attempt.
use std::fmt;

/// Status of a module's latest request for the selected locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleStatus {
	/// No current request needs the module.
	Unloaded,
	/// Loading or retrying. A previous valid same-locale snapshot may still exist.
	Loading,
	/// The latest candidate loaded successfully.
	Ready,
	/// The latest attempt failed; a previous valid same-locale snapshot may remain.
	Failed(String),
}

/// A typed scope is unavailable for the selected locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleError {
	/// Selected language code.
	pub locale: String,
	/// First required logical leaf that is unavailable.
	pub path: &'static str,
	/// Its current request state.
	pub status: ModuleStatus,
}

impl fmt::Display for ModuleError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			formatter,
			"{}/{} is unavailable ({:?})",
			self.locale, self.path, self.status
		)
	}
}

impl std::error::Error for ModuleError {}
