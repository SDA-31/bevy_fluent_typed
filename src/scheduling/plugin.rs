use super::{StartupStrategy, configuration::StartupConfiguration, lifecycle};
use crate::bevy::prelude::{App, Plugin, SystemSet};

/// Configure startup coordination independently of localization source plugins.
///
/// Available on Bevy 0.19/0.20. Register this plugin before the first App update,
/// either before or after `LocalizationPlugin`. Unconfigured localization roots
/// add an implicit Deferred fallback when this plugin is absent. An explicit
/// plugin added later replaces that fallback's policy, without adding another
/// coordinator or replaying startup. Matching explicit choices are accepted;
/// contradictory explicit choices are rejected. Localization source plugins
/// inherit this App-wide policy and do not configure it themselves.
pub struct LocalizationStartupPlugin {
	configuration: Option<StartupConfiguration>,
}

impl LocalizationStartupPlugin {
	/// Select Native or Deferred startup for this App.
	///
	/// For a selected-set barrier, use [`Self::barrier`].
	pub fn new(strategy: StartupStrategy) -> Self {
		Self {
			configuration: Some(StartupConfiguration::new(strategy)),
		}
	}

	/// Wait for the combined catalog requirements of a selected startup set.
	///
	/// Independent systems outside the set can proceed, subject to declared ordering.
	/// Loading mode is unchanged; Manual still requires application-owned requests.
	pub fn barrier(set: impl SystemSet) -> Self {
		Self {
			configuration: Some(StartupConfiguration::barrier(set.intern())),
		}
	}

	pub(crate) fn fallback() -> Self {
		Self {
			configuration: None,
		}
	}
}

impl Plugin for LocalizationStartupPlugin {
	fn build(&self, app: &mut App) {
		lifecycle::register(app, self.configuration);
	}

	fn is_unique(&self) -> bool {
		// An explicit plugin may follow the automatically registered fallback.
		// The shared coordinator rejects conflicting policies and installs once.
		false
	}
}
