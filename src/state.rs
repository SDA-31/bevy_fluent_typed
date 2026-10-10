//! Selected locale and scope demand, independent of physical asset handles.
#[cfg(feature = "manifest")]
use crate::assets::ModuleAsset;
#[cfg(feature = "manifest")]
use crate::bevy::prelude::Handle;
use crate::bevy::{ecs as bevy_ecs, prelude::Resource, tasks::Task};
use crate::leases::LeaseRequests;
use crate::{
	Auto, FluentCatalog, FluentScope, Lazy, LoadingMode, ModuleLease, ModuleStatus, ModuleStore,
};
use std::{
	any::TypeId,
	collections::{BTreeMap, BTreeSet, HashMap},
	marker::PhantomData,
	sync::Arc,
};

type RetryRequests = crate::progress::ObservedSet;

pub(crate) struct RequestedModule<C: FluentCatalog> {
	pub(crate) locale: C::Locale,
	pub(crate) task: Option<Task<Result<crate::catalog::SharedScope, String>>>,
	#[cfg(feature = "manifest")]
	pub(crate) handle: Option<Handle<ModuleAsset<C>>>,
	#[cfg(feature = "manifest")]
	pub(crate) accepted: Option<u64>,
	#[cfg(feature = "manifest")]
	pub(crate) preparation_request: Option<u64>,
	#[cfg(feature = "manifest")]
	pub(crate) preparation_handle: Option<Handle<crate::assets::PreparedModuleAsset<C>>>,
	#[cfg(feature = "manifest")]
	pub(crate) preparation_attempt: Option<Arc<crate::assets::AssetAttempt>>,
	// Tracks our request before AssetServer's detached task changes load state.
	pub(crate) pending: bool,
}

impl<C: FluentCatalog> RequestedModule<C> {
	pub(crate) fn new(locale: C::Locale) -> Self {
		Self {
			locale,
			task: None,
			#[cfg(feature = "manifest")]
			handle: None,
			#[cfg(feature = "manifest")]
			accepted: None,
			#[cfg(feature = "manifest")]
			preparation_request: None,
			#[cfg(feature = "manifest")]
			preparation_handle: None,
			#[cfg(feature = "manifest")]
			preparation_attempt: None,
			pending: false,
		}
	}
}

/// Controller for the selected locale and requested scopes.
///
/// Default Auto tracks inserted bindings and localized required-resource systems.
/// Optional manual pins retain additional scopes independently of consumers.
/// With neither consumers nor pins, no modules are loaded. Full requests the root;
/// explicit Lazy exposes idempotent application-owned scope requests.
/// Independent ownership leases can retain scopes alongside manual pins and consumers.
/// Resource publication is synchronized at the plugin's Publish/Refresh boundaries.
#[derive(Resource)]
pub struct Localization<C: FluentCatalog, M: LoadingMode = Auto> {
	pub(crate) store: ModuleStore<C>,
	pub(crate) requested: HashMap<TypeId, &'static [&'static str]>,
	pub(crate) entries: BTreeMap<&'static str, RequestedModule<C>>,
	pub(crate) retry: RetryRequests,
	pub(crate) published: HashMap<TypeId, u64>,
	leases: LeaseRequests,
	pub(crate) desired: Arc<BTreeSet<&'static str>>,
	automatic: Arc<BTreeSet<&'static str>>,
	pub(crate) automatic_revision: u64,
	pub(crate) synchronized: u64,
	pub(crate) requests_changed: bool,
	pub(crate) pending: usize,
	marker: PhantomData<fn() -> M>,
	pub(crate) preparation: Option<Box<Self>>,
	pub(crate) staged: bool,
	pub(crate) commit_requested: bool,
	pub(crate) switch_when_ready: bool,
	#[cfg(feature = "manifest")]
	pub(crate) handoff_pending: BTreeSet<&'static str>,
}

impl<C: FluentCatalog, M: LoadingMode> Default for Localization<C, M> {
	fn default() -> Self {
		Self::new(C::default_locale())
	}
}

impl<C: FluentCatalog, M: LoadingMode> Localization<C, M> {
	/// Initialize an empty controller with a selected compiled locale.
	/// No manifest is read and no Fluent resource is constructed here.
	///
	/// # Panics
	/// Panics if the locale is not declared by the provider.
	pub fn new(locale: C::Locale) -> Self {
		assert!(
			C::locales().contains(&locale),
			"locale must belong to the provider"
		);
		let mut requested = HashMap::new();

		if M::FULL {
			requested.insert(TypeId::of::<C>(), C::module_paths());
		}

		let desired = requested
			.values()
			.flat_map(|paths| paths.iter().copied())
			.collect();

		Self {
			store: ModuleStore::new(locale),
			requested,
			entries: BTreeMap::new(),
			retry: RetryRequests::new(),
			published: HashMap::new(),
			leases: LeaseRequests::default(),
			desired: Arc::new(desired),
			automatic: Arc::default(),
			automatic_revision: u64::MAX,
			synchronized: u64::MAX,
			requests_changed: true,
			pending: 0,
			marker: PhantomData,
			preparation: None,
			staged: false,
			commit_requested: false,
			switch_when_ready: false,
			#[cfg(feature = "manifest")]
			handoff_pending: BTreeSet::new(),
		}
	}

	/// Active language, retained until a requested locale is ready and published.
	pub fn locale(&self) -> C::Locale {
		self.store.locale
	}

	/// Prepare a locale and automatically publish it when all desired scopes are ready.
	/// Existing resources and text remain on the active locale while preparation runs.
	/// Selecting the active locale cancels a pending switch; the latest target wins.
	/// Repeating a failed target retries it without adding a per-frame retry loop.
	/// Selecting the active locale retries failed desired active leaves, without
	/// pinning scopes or reloading ready leaves. Target failures preserve active
	/// resources and report the target locale through `CatalogUpdate::Rejected`.
	/// Before the first app update, selection chooses only the initial language.
	/// Use `prepared_locale`/`preparation_status` to inspect an in-flight target.
	///
	/// # Panics
	/// Panics if the locale is not declared by the provider.
	pub fn set_locale(&mut self, locale: C::Locale) {
		assert!(
			C::locales().contains(&locale),
			"locale must belong to the provider"
		);

		if self.locale() == locale {
			self.cancel_preparation();

			for &path in self.desired.iter() {
				if matches!(self.store.states.get(path), Some(ModuleStatus::Failed(_))) {
					self.retry.insert(path);
				}
			}

			return;
		}

		// Before the first publication/acquisition, choose only the initial target.
		// A missing root is insufficient: existing leaf snapshots must stay usable.
		if self.synchronized == u64::MAX
			&& self.store.values.is_empty()
			&& self.published.is_empty()
			&& self.entries.is_empty()
		{
			self.cancel_preparation();
			self.store.locale = locale;
			self.requests_changed = true;
			return;
		}

		self.prepare_locale(locale);
		self.switch_when_ready = true;
		self.preparation
			.as_mut()
			.expect("requested locale preparation")
			.switch_when_ready = true;
	}

	/// Borrow the root when every selected-language module is ready.
	pub fn catalog(&self) -> Option<&C> {
		self.store.get::<C>().ok()
	}

	/// Navigate known groups and access ready leaves without starting I/O.
	pub fn modules(&self) -> C::Modules<'_> {
		C::view(&self.store)
	}

	/// Inspect the latest loading attempt across a scope's required leaves.
	pub fn status<S: FluentScope<Catalog = C>>(&self) -> ModuleStatus {
		self.store.status::<S>()
	}

	/// Inspect a scope's active-locale attempts without requesting or loading it.
	/// Includes unrequested schema leaves. Native root `LocalizationProgress`
	/// snapshots instead count the current demand union; group and leaf views count
	/// their fixed schema.
	pub fn progress<S: FluentScope<Catalog = C>>(&self) -> crate::LoadingProgress<C::Locale> {
		self.store.progress::<S>()
	}

	/// Inspect a scope in the current preparation without publishing target data.
	/// Returns `None` without preparation. Preparing the active locale mirrors
	/// active attempts; use `preparation_status` to decide whether commit is ready.
	pub fn preparation_progress<S: FluentScope<Catalog = C>>(
		&self,
	) -> Option<crate::LoadingProgress<C::Locale>> {
		self.preparation.as_deref().map(|target| {
			if target.locale() == self.locale() {
				self.store.progress::<S>()
			} else {
				target.store.progress::<S>()
			}
		})
	}

	/// Inspect active-locale module details without requesting or retaining a scope.
	/// Available with `diagnostics`; collects its result only when called.
	#[cfg(feature = "diagnostics")]
	pub fn diagnostics<S: FluentScope<Catalog = C>>(&self) -> Vec<crate::ModuleDiagnostic> {
		self.store.diagnostics::<S>()
	}

	/// Inspect target module details without publishing target data.
	/// Available with `diagnostics`; returns `None` when no preparation exists.
	#[cfg(feature = "diagnostics")]
	pub fn preparation_diagnostics<S: FluentScope<Catalog = C>>(
		&self,
	) -> Option<Vec<crate::ModuleDiagnostic>> {
		self.preparation.as_deref().map(|target| {
			if target.locale() == self.locale() {
				self.store.diagnostics::<S>()
			} else {
				target.store.diagnostics::<S>()
			}
		})
	}

	pub(crate) fn desired(&self) -> Arc<BTreeSet<&'static str>> {
		self.desired.clone()
	}

	pub(crate) fn has_dropped_leases(&self) -> bool {
		self.leases.has_releases()
	}

	pub(crate) fn release_dropped_leases(&mut self) {
		if self.leases.release_dropped() {
			self.requests_changed = true;
			self.rebuild_desired();
		}
	}

	pub(crate) fn set_automatic(&mut self, paths: Arc<BTreeSet<&'static str>>, revision: u64) {
		self.automatic_revision = revision;

		if self.automatic == paths {
			return;
		}

		self.automatic = paths;
		self.requests_changed = true;
		self.rebuild_desired();
	}

	fn rebuild_desired(&mut self) {
		self.desired = Arc::new(
			self.requested
				.values()
				.copied()
				.chain(self.leases.paths())
				.flat_map(|paths| paths.iter().copied())
				.chain(self.automatic.iter().copied())
				.collect(),
		);
	}

	pub(crate) fn finish_request(&mut self, path: &str) -> &mut RequestedModule<C> {
		let entry = self
			.entries
			.get_mut(path)
			.expect("registered module request");

		if std::mem::replace(&mut entry.pending, false) {
			self.pending -= 1;
		}

		entry
	}
}

impl<C: FluentCatalog, M: LoadingMode> Localization<C, M> {
	fn hold_explicit<S: FluentScope<Catalog = C>>(&mut self) -> ModuleLease<S> {
		let lease = self.leases.hold::<S>();

		self.requests_changed = true;
		Arc::make_mut(&mut self.desired).extend(S::module_paths().iter().copied());
		self.synchronize_preparation_requests();

		lease
	}

	fn load_explicit<S: FluentScope<Catalog = C>>(&mut self) {
		if self
			.requested
			.insert(TypeId::of::<S>(), S::module_paths())
			.is_none()
		{
			self.requests_changed = true;
			Arc::make_mut(&mut self.desired).extend(S::module_paths().iter().copied());
		}

		self.synchronize_preparation_requests();

		for &path in S::module_paths() {
			if matches!(self.store.states.get(path), Some(ModuleStatus::Failed(_))) {
				self.retry.insert(path);
			}
		}

		if let Some(preparation) = self.preparation.as_mut() {
			for &path in S::module_paths() {
				if matches!(
					preparation.store.states.get(path),
					Some(ModuleStatus::Failed(_))
				) {
					preparation.retry.insert(path);
					self.commit_requested = false;
				}
			}
		}
	}

	/// Release this explicit request. Independent leases and overlapping requests remain active.
	/// The plugin releases unneeded resources and strong handles at synchronization.
	fn unload_explicit<S: FluentScope<Catalog = C>>(&mut self) {
		if self.requested.remove(&TypeId::of::<S>()).is_some() {
			self.requests_changed = true;
			self.rebuild_desired();
		}

		self.synchronize_preparation_requests();
	}
}

macro_rules! explicit_requests {
	($($mode:ty),+) => {
		$(impl<C: FluentCatalog> Localization<C, $mode> {
			/// Retain an independent scope request until its token drops.
			/// Manual pins, other leases and automatic consumers remain independent.
			/// Acquiring a lease does not retry failures; use `load` or `ReloadCatalogs`.
			pub fn hold<S: FluentScope<Catalog = C>>(&mut self) -> ModuleLease<S> {
				self.hold_explicit::<S>()
			}

			/// Keep a scope loaded independently of automatic consumers until `unload`.
			/// Repeated calls are idempotent: one `unload` removes the manual pin.
			/// Repeating a failed request retries active and prepared leaves.
			pub fn load<S: FluentScope<Catalog = C>>(&mut self) {
				self.load_explicit::<S>();
			}

			/// Remove only the manual pin; automatic and overlapping owners stay active.
			/// Unneeded resources and handles release at the next synchronization.
			pub fn unload<S: FluentScope<Catalog = C>>(&mut self) {
				self.unload_explicit::<S>();
			}
		})+
	};
}

explicit_requests!(Auto, Lazy);
