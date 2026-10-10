//! Latest native Bevy snapshot, separate from the controller's live state.
use super::{LoadingProgress, version::Stamp};
use crate::bevy::{ecs as bevy_ecs, prelude::Resource};
use crate::{FluentCatalog, FluentScope, PreparationStatus};

/// Native latest progress for one explicitly observed root, group or leaf.
///
/// Enable with `LocalizationProgressPlugin<S>` through `App::add_plugins`. Once its
/// owning localization plugin is installed, this resource exists before `Startup`.
/// Inspect with native `Res` and `resource_changed`; tracking never requests
/// catalogs. Publication follows reconciliation in PreUpdate and PostUpdate.
/// Multiple transitions between observations may coalesce.
///
/// Root snapshots count current demand: Lazy counts automatic consumers, manual
/// pins and independent leases; Full counts every module; Manual counts pins and
/// leases. Group/leaf snapshots count their fixed
/// unique schema paths, including unrequested files, without starting I/O.
/// Preparation readiness describes the entire provider's demand, independently
/// of this view's counters, because a locale commit applies to the provider.
#[derive(Resource)]
pub struct LocalizationProgress<S: FluentScope> {
	pub(super) active: LoadingProgress<<S::Catalog as FluentCatalog>::Locale>,
	pub(super) preparation: Option<LoadingProgress<<S::Catalog as FluentCatalog>::Locale>>,
	pub(super) status: PreparationStatus,
	pub(super) stamp: Stamp<<S::Catalog as FluentCatalog>::Locale>,
}

impl<S: FluentScope> LocalizationProgress<S> {
	/// Latest published progress of the active locale, including ordinary Full loading.
	pub fn active(&self) -> &LoadingProgress<<S::Catalog as FluentCatalog>::Locale> {
		&self.active
	}

	/// Latest target progress, or `None` when no preparation exists.
	/// Preparing the active locale mirrors active data and starts no extra I/O.
	pub fn preparation(&self) -> Option<&LoadingProgress<<S::Catalog as FluentCatalog>::Locale>> {
		self.preparation.as_ref()
	}

	/// Provider-wide readiness of the prepared target at this publication boundary.
	/// A parsed target can still be pending due to retries or asset-handle handoff;
	/// `ready == total` is not a replacement for `PreparationStatus::Ready`.
	/// Use the live controller's `commit_locale` for validation when committing.
	pub fn preparation_status(&self) -> &PreparationStatus {
		&self.status
	}

	pub(super) fn same_visible_state(&self, other: &Self) -> bool {
		self.active == other.active
			&& self.preparation == other.preparation
			&& self.status == other.status
	}
}
