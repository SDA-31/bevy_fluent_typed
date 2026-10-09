//! Selected locale and explicit logical demand, independent of physical asset handles.
#[cfg(feature = "manifest")]
use crate::assets::ModuleAsset;
#[cfg(feature = "manifest")]
use crate::bevy::prelude::Handle;
use crate::bevy::{ecs as bevy_ecs, prelude::Resource, tasks::Task};
use crate::{
	FluentCatalog, FluentScope, Full, Lazy, LoadingMode, LoadingProgress, ModuleStatus, ModuleStore,
};
use std::{
	any::TypeId,
	collections::{BTreeMap, BTreeSet, HashMap},
	marker::PhantomData,
	sync::Arc,
};

pub(crate) struct RequestedModule<C: FluentCatalog> {
	pub(crate) locale: C::Locale,
	pub(crate) task: Option<Task<Result<crate::catalog::SharedScope, String>>>,
	#[cfg(feature = "manifest")]
	pub(crate) handle: Option<Handle<ModuleAsset<C>>>,
	#[cfg(feature = "manifest")]
	pub(crate) accepted: Option<u64>,
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
			pending: false,
		}
	}
}

/// Controller for the selected locale and the scopes requested by the application.
///
/// Full mode always requests the root. Lazy mode exposes idempotent scope requests.
/// Resource publication is synchronized at the plugin's Publish/Refresh boundaries.
#[derive(Resource)]
pub struct Localization<C: FluentCatalog, M: LoadingMode = Full> {
	pub(crate) store: ModuleStore<C>,
	pub(crate) requested: HashMap<TypeId, &'static [&'static str]>,
	pub(crate) entries: BTreeMap<&'static str, RequestedModule<C>>,
	pub(crate) retry: BTreeSet<&'static str>,
	pub(crate) published: HashMap<TypeId, u64>,
	desired: Arc<BTreeSet<&'static str>>,
	pub(crate) synchronized: u64,
	pub(crate) requests_changed: bool,
	pub(crate) pending: usize,
	marker: PhantomData<fn() -> M>,
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
			retry: BTreeSet::new(),
			published: HashMap::new(),
			desired: Arc::new(desired),
			synchronized: u64::MAX,
			requests_changed: true,
			pending: 0,
			marker: PhantomData,
		}
	}

	/// Currently selected language, including during asynchronous transitions.
	pub fn locale(&self) -> C::Locale {
		self.store.locale
	}

	/// Select a locale while retaining logical scope requests.
	/// When the locale changes, controller snapshots clear immediately.
	/// Published scope resources synchronize at the next publication boundary.
	///
	/// # Panics
	/// Panics if the locale is not declared by the provider.
	pub fn set_locale(&mut self, locale: C::Locale) {
		assert!(
			C::locales().contains(&locale),
			"locale must belong to the provider"
		);

		if self.locale() == locale {
			return;
		}

		self.store.locale = locale;
		self.requests_changed = true;
		self.store.revision += 1;
		self.store.values.clear();
		self.store.states.clear();
		self.entries.clear();
		self.pending = 0;
		self.retry.clear();
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

	/// Inspect a scope's unique leaves, latest attempts and usable snapshots.
	/// Includes unrequested leaves without starting I/O or changing logical demand.
	pub fn progress<S: FluentScope<Catalog = C>>(&self) -> LoadingProgress<C::Locale> {
		self.store.progress::<S>()
	}

	pub(crate) fn desired(&self) -> Arc<BTreeSet<&'static str>> {
		self.desired.clone()
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

impl<C: FluentCatalog> Localization<C, Lazy> {
	/// Request a leaf, complete group or complete root until explicitly unloaded.
	/// Repeated requests are idempotent, not reference counted: one `unload::<S>()`
	/// releases any number of earlier `load::<S>()` calls for that scope type.
	/// A repeated failed request retries its leaves.
	pub fn load<S: FluentScope<Catalog = C>>(&mut self) {
		if self
			.requested
			.insert(TypeId::of::<S>(), S::module_paths())
			.is_none()
		{
			self.requests_changed = true;
			Arc::make_mut(&mut self.desired).extend(S::module_paths().iter().copied());
		}

		for &path in S::module_paths() {
			if matches!(self.store.states.get(path), Some(ModuleStatus::Failed(_))) {
				self.retry.insert(path);
			}
		}
	}

	/// Release this explicit request. Independent overlapping requests remain active.
	/// The plugin releases unneeded resources and strong handles at synchronization.
	pub fn unload<S: FluentScope<Catalog = C>>(&mut self) {
		if self.requested.remove(&TypeId::of::<S>()).is_some() {
			self.requests_changed = true;
			self.desired = Arc::new(
				self.requested
					.values()
					.flat_map(|paths| paths.iter().copied())
					.collect(),
			);
		}
	}
}
