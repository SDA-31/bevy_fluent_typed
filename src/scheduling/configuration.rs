use crate::bevy::ecs::schedule::InternedSystemSet;

/// How native startup systems wait for their localization catalogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartupStrategy {
	/// Preserve Bevy's ordinary one-shot startup and missing-resource validation.
	Native,
	/// Run independent startup steps once, retrying catalog waits and dependents.
	#[default]
	Deferred,
	/// Wait for every catalog needed by the set selected with `with_startup_barrier`.
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
