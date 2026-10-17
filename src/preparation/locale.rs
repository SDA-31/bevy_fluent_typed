//! Private locale staging for automatic switching and manual publication control.
use crate::bevy::prelude::*;
use crate::{FluentCatalog, LoadingMode, Localization, ModuleError, ModuleStatus};
use std::fmt;
#[cfg(feature = "manifest")]
use std::sync::atomic::{AtomicU64, Ordering};

/// Availability of the target locale, independent of active catalog readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparationStatus {
	/// No locale preparation exists.
	Idle,
	/// Some currently requested target leaves are pending or scheduled for retry.
	Preparing,
	/// Every currently requested target leaf passed its latest validation attempt.
	/// File-source handles are also ready for a safe handoff.
	Ready,
	/// A requested target leaf failed its latest attempt.
	Failed(ModuleError),
}

/// A locale commit could not be scheduled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitLocaleError {
	/// Call `prepare_locale` before committing.
	NotPrepared,
	/// The target still has pending leaves, retries or file-handle retirement.
	Pending,
	/// A target leaf failed validation or acquisition.
	Failed(ModuleError),
}

impl fmt::Display for CommitLocaleError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::NotPrepared => formatter.write_str("no locale is being prepared"),
			Self::Pending => formatter.write_str("locale preparation is still pending"),
			Self::Failed(error) => write!(formatter, "locale preparation failed: {error}"),
		}
	}
}

impl std::error::Error for CommitLocaleError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Failed(error) => Some(error),
			Self::NotPrepared | Self::Pending => None,
		}
	}
}

impl<C: FluentCatalog, M: LoadingMode> Localization<C, M> {
	/// Prepare the currently requested scopes in another locale without publishing them.
	/// Repeating the target is idempotent and retries failed leaves. A different target
	/// replaces the previous preparation. Preparing the active locale performs no I/O.
	/// This advanced call takes over manual publication control, even for a target
	/// already requested by `set_locale`; automatic switching stops until requested again.
	/// Preparation follows the current consumers and manual pins in Lazy, manual
	/// requests in Manual, or all modules in Full. Manual failures are inspected
	/// through `preparation_status`, without `Rejected` notifications; successful
	/// leaves publish on commit. Active resources remain available throughout.
	///
	/// # Panics
	/// Panics if the locale is not declared by the provider.
	pub fn prepare_locale(&mut self, locale: C::Locale) {
		assert!(
			C::locales().contains(&locale),
			"locale must belong to the provider"
		);
		self.switch_when_ready = false;

		if let Some(preparation) = self.preparation.as_mut() {
			preparation.switch_when_ready = false;
		}

		if let Some(preparation) = self.preparation.as_mut()
			&& preparation.locale() == locale
		{
			for &path in self.desired.iter() {
				if matches!(
					preparation.store.states.get(path),
					Some(ModuleStatus::Failed(_))
				) {
					preparation.retry.insert(path);
					self.commit_requested = false;
				}
			}

			return;
		}

		let mut preparation = Self::new(locale);
		preparation.staged = true;
		preparation.requested = self.requested.clone();
		preparation.desired = self.desired();
		self.preparation = Some(Box::new(preparation));
		self.commit_requested = false;
	}

	/// Target locale, including while it is still being prepared.
	pub fn prepared_locale(&self) -> Option<C::Locale> {
		self.preparation
			.as_ref()
			.map(|preparation| preparation.locale())
	}

	/// Inspect the latest attempt for every currently requested target leaf.
	/// An empty Lazy/Manual demand is ready without acquiring any modules.
	pub fn preparation_status(&self) -> PreparationStatus {
		let Some(preparation) = self.preparation.as_ref() else {
			return PreparationStatus::Idle;
		};

		if preparation.locale() == self.locale() {
			return PreparationStatus::Ready;
		}

		let mut pending = preparation.pending > 0 || !preparation.retry.is_empty();
		#[cfg(feature = "manifest")]
		{
			pending |= !preparation.handoff_pending.is_empty();
		}

		for &path in self.desired.iter() {
			match preparation.store.states.get(path) {
				Some(ModuleStatus::Failed(error)) if !preparation.retry.contains(path) => {
					return PreparationStatus::Failed(ModuleError {
						locale: preparation.locale().as_ref().into(),
						path,
						status: ModuleStatus::Failed(error.clone()),
					});
				}
				Some(ModuleStatus::Ready)
					if preparation
						.store
						.leaves
						.get(path)
						.is_some_and(|id| preparation.store.values.contains_key(id)) => {}
				_ => pending = true,
			}
		}

		if pending {
			PreparationStatus::Preparing
		} else {
			PreparationStatus::Ready
		}
	}

	/// Schedule a validated target for the next PreUpdate Publish boundary.
	/// Active catalogs and `locale()` remain unchanged until that boundary.
	/// Changes to scope demand or explicit retries revoke a queued commit; commit again
	/// after preparation is ready. Automatic `set_locale` intent instead continues
	/// through changes and retries without a separate commit call. Publication
	/// rechecks source readiness.
	///
	/// # Errors
	/// Returns a typed error when preparation is absent, pending or failed.
	pub fn commit_locale(&mut self) -> Result<(), CommitLocaleError> {
		match self.preparation_status() {
			PreparationStatus::Idle => Err(CommitLocaleError::NotPrepared),
			PreparationStatus::Preparing => Err(CommitLocaleError::Pending),
			PreparationStatus::Failed(error) => Err(CommitLocaleError::Failed(error)),
			PreparationStatus::Ready => {
				self.commit_requested = true;
				Ok(())
			}
		}
	}

	/// Cancel manual preparation or pending automatic switching; keep the active locale.
	/// Drops target snapshots and tasks/handles and cancels queued publication.
	/// External acquisition may continue, but its completion cannot publish a target.
	pub fn cancel_preparation(&mut self) {
		self.preparation = None;
		self.commit_requested = false;
		self.switch_when_ready = false;
	}

	pub(crate) fn synchronize_preparation_requests(&mut self) {
		let Some(preparation) = self.preparation.as_mut() else {
			return;
		};

		if preparation.desired == self.desired {
			return;
		}

		preparation.requested.clone_from(&self.requested);
		preparation.desired = self.desired.clone();
		preparation.requests_changed = true;
		crate::loading::release_unrequested(preparation);
		self.commit_requested = false;
	}
}

pub(crate) fn commit<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	let localization = world.resource::<Localization<C, M>>();

	if !localization.commit_requested && !localization.switch_when_ready {
		return;
	}

	// A pending or failed automatic target is not a controller mutation. File
	// reconciliation continues polling retiring handles before this boundary.
	if localization.preparation_status() != PreparationStatus::Ready {
		if localization.commit_requested {
			world.resource_mut::<Localization<C, M>>().commit_requested = false;
		}

		return;
	}

	world.resource_scope(|world, mut localization: Mut<Localization<C, M>>| {
		#[cfg(feature = "manifest")]
		if localization
			.prepared_locale()
			.is_some_and(|locale| locale != localization.locale())
			&& let Some(source) = world.get_resource::<crate::loading::CatalogSource<C>>()
			&& let Some(server) = world.get_resource::<AssetServer>()
			&& let Some(preparation) = localization.preparation.as_mut()
		{
			refresh_handoff(preparation, source, server);
		}

		if localization.preparation_status() != PreparationStatus::Ready {
			if localization.commit_requested {
				localization.commit_requested = false;
			}

			return;
		}

		let mut preparation = localization
			.preparation
			.take()
			.expect("validated preparation");
		localization.commit_requested = false;
		localization.switch_when_ready = false;

		if preparation.locale() == localization.locale() {
			return;
		}

		// Target leaf revisions start at zero. Rebase them above every active
		// revision so native resources and deferred binding signatures cannot alias.
		let offset = localization.store.revision + 1;
		preparation.store.revision += offset;

		for value in preparation.store.values.values_mut() {
			value.revision += offset;

			for (_, revision) in &mut value.signature {
				*revision += offset;
			}
		}

		#[cfg(feature = "manifest")]
		if let Some(source) = world.get_resource::<crate::loading::CatalogSource<C>>()
			&& let Some(server) = world.get_resource::<AssetServer>()
		{
			for (&path, entry) in &mut preparation.entries {
				if entry.preparation_handle.is_some()
					&& let Ok(address) = crate::loading::asset_address(
						&source.manifest,
						preparation.store.locale.as_ref(),
						path,
					) {
					// Return to the normal asset namespace for watching. Abandoned
					// preparation assets cannot affect these active handles.
					entry.handle = Some(server.load(address));
					// Serialize explicit reloads behind this read as well, while
					// keeping the committed snapshot available to consumers.
					entry.pending = true;
					preparation.pending += 1;
					entry.preparation_handle = None;
					entry.preparation_request = None;
					entry.preparation_attempt = None;
				}
			}
		}

		localization.store = preparation.store;
		localization.entries = preparation.entries;
		localization.retry = preparation.retry;
		localization.pending = preparation.pending;
		localization.requests_changed = false;

		for &path in localization.desired.iter() {
			let update = crate::CatalogUpdate::<C>::Loaded {
				locale: localization.locale(),
				path: path.into(),
			};
			crate::compatibility::write_update(world, update);
		}
	});

	// Keep the controller locale and native scope resources coherent even for
	// another PreUpdate system scheduled immediately after this exclusive step.
	crate::resources::synchronize::<C, M>(world);
}

#[cfg(feature = "manifest")]
static NEXT_ASSET_REQUEST: AtomicU64 = AtomicU64::new(1);

#[cfg(feature = "manifest")]
pub(crate) fn next_asset_request() -> u64 {
	NEXT_ASSET_REQUEST.fetch_add(1, Ordering::Relaxed)
}

#[cfg(feature = "manifest")]
pub(crate) fn load_asset<C: FluentCatalog>(
	server: &AssetServer,
	address: crate::bevy::asset::AssetPath<'static>,
	request: u64,
	attempts: &crate::assets::PreparationAttempts<C>,
) -> (
	Handle<crate::assets::PreparedModuleAsset<C>>,
	std::sync::Arc<crate::assets::AssetAttempt>,
) {
	let attempt = attempts.register(request);
	let applied = std::sync::Arc::downgrade(&attempt);
	let settings = move |settings: &mut (u64, ())| {
		*settings = (request, ());

		if let Some(applied) = applied.upgrade() {
			applied.settings_applied.store(true, Ordering::Release);
		}
	};
	let guard = crate::assets::PreparationGuard(std::sync::Arc::downgrade(&attempt));
	let handle = crate::compatibility::load_with_settings(server, address, settings, guard);
	(handle, attempt)
}

#[cfg(feature = "manifest")]
pub(crate) fn refresh_handoff<C: FluentCatalog, M: LoadingMode>(
	preparation: &mut Localization<C, M>,
	source: &crate::loading::CatalogSource<C>,
	server: &AssetServer,
) {
	preparation.handoff_pending.clear();

	if source.manifest.embedded_modules().is_some() {
		return;
	}

	for &path in preparation.desired.iter() {
		let Ok(address) =
			crate::loading::asset_address(&source.manifest, preparation.locale().as_ref(), path)
		else {
			continue;
		};

		// A canceled normal read can still own a path handle before loader entry.
		// Wait for its identity to retire before commit creates a normal handle;
		// otherwise server.load() reconnects that obsolete reader and result.
		if server
			.get_path_ids(address)
			.iter()
			.any(|id| id.type_id() == std::any::TypeId::of::<crate::assets::ModuleAsset<C>>())
		{
			preparation.handoff_pending.insert(path);
		}
	}

	if !preparation.handoff_pending.is_empty() {
		preparation.requests_changed = true;
	}
}
