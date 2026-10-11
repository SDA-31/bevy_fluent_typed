use crate::bevy::ecs::schedule::InternedSystemSet;

/// How native startup systems wait for their localization catalogs on Bevy 0.19/0.20.
///
/// Selected on the existing `LocalizationPlugin`; unconfigured roots share the
/// App's explicit policy, or Deferred when no root makes an explicit choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartupStrategy {
	/// Preserve Bevy's ordinary one-shot startup and missing-resource validation.
	Native,
	/// Run independent startup steps once, retrying catalog waits and dependents.
	#[default]
	Deferred,
	/// Wait for every catalog needed by the set selected with `with_startup_barrier`.
	/// Use that builder rather than passing this variant to `with_startup_strategy`.
	Barrier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StartupConfiguration {
	pub(crate) strategy: StartupStrategy,
	pub(crate) barrier: Option<InternedSystemSet>,
}

impl StartupConfiguration {
	pub(crate) fn new(strategy: StartupStrategy) -> Self {
		assert!(
			!matches!(strategy, StartupStrategy::Barrier),
			"Barrier requires a selected startup set; use with_startup_barrier(set)"
		);
		Self {
			strategy,
			barrier: None,
		}
	}

	pub(crate) fn barrier(set: InternedSystemSet) -> Self {
		Self {
			strategy: StartupStrategy::Barrier,
			barrier: Some(set),
		}
	}
}

impl Default for StartupConfiguration {
	fn default() -> Self {
		Self::new(StartupStrategy::Deferred)
	}
}
