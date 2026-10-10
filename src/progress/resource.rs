//! Latest native Bevy snapshot, separate from the controller's live state.
use super::{LoadingProgress, publication::Stamp};
use crate::bevy::{ecs as bevy_ecs, prelude::Resource};
use crate::{FluentCatalog, PreparationStatus};

/// Native latest progress for one provider with explicitly enabled tracking.
///
/// Enable with `LocalizationAppExt::add_localization_progress::<C>()`. Once its
/// plugin is installed, the helper inserts this resource before `Startup`. Inspect with native
/// `Res` and `resource_changed`; no manual resource registration or catalog demand
/// is required beyond that explicit opt-in. Publication follows reconciliation in PreUpdate and
/// PostUpdate. Multiple transitions between observations may coalesce.
///
/// Active and prepared snapshots count the current union of requested modules:
/// Full counts the root; Lazy counts explicit pins and independent leases.
/// Use controller/store per-scope `progress` queries to inspect the full schema
/// of another scope, including its unrequested modules.
#[derive(Resource)]
pub struct LocalizationProgress<C: FluentCatalog> {
	pub(super) active: LoadingProgress<C::Locale>,
	pub(super) preparation: Option<LoadingProgress<C::Locale>>,
	pub(super) status: PreparationStatus,
	pub(super) stamp: Stamp<C::Locale>,
}

impl<C: FluentCatalog> LocalizationProgress<C> {
	/// Latest published progress of the active locale, including ordinary Full loading.
	pub fn active(&self) -> &LoadingProgress<C::Locale> {
		&self.active
	}

	/// Latest target progress, or `None` when no preparation exists.
	/// Preparing the active locale mirrors active data and starts no extra I/O.
	pub fn preparation(&self) -> Option<&LoadingProgress<C::Locale>> {
		self.preparation.as_ref()
	}

	/// Authoritative readiness of the prepared target at this publication boundary.
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
