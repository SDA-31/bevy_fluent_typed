//! Selected locale and explicit logical demand, independent of physical asset handles.
use crate::assets::ModuleAsset;
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{Handle, Resource},
};
use crate::{FluentCatalog, FluentScope, Full, Lazy, LoadingMode, ModuleStatus, ModuleStore};
use std::{
	any::TypeId,
	collections::{BTreeMap, BTreeSet, HashMap},
	marker::PhantomData,
};

pub(crate) struct RequestedModule<C: FluentCatalog> {
	pub(crate) handle: Option<Handle<ModuleAsset<C>>>,
	pub(crate) accepted: Option<u64>,
	// Tracks our request before AssetServer's detached task changes load state.
	pub(crate) pending: bool,
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

		Self {
			store: ModuleStore::new(locale),
			requested,
			entries: BTreeMap::new(),
			retry: BTreeSet::new(),
			published: HashMap::new(),
			marker: PhantomData,
		}
	}

	/// Currently selected language, including during asynchronous transitions.
	pub fn locale(&self) -> C::Locale {
		self.store.locale
	}

	/// Select a locale while retaining logical scope requests.
	/// Previous-language snapshots are removed at the next publication boundary.
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
		self.store.values.clear();
		self.store.states.clear();
		self.entries.clear();
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

	pub(crate) fn desired(&self) -> BTreeSet<&'static str> {
		self.requested
			.values()
			.flat_map(|paths| paths.iter().copied())
			.collect()
	}
}

impl<C: FluentCatalog> Localization<C, Lazy> {
	/// Request a leaf, complete group or complete root until explicitly unloaded.
	/// Repeated requests are idempotent; a repeated failed request retries its leaves.
	pub fn load<S: FluentScope<Catalog = C>>(&mut self) {
		self.requested.insert(TypeId::of::<S>(), S::module_paths());

		for &path in S::module_paths() {
			if matches!(self.store.states.get(path), Some(ModuleStatus::Failed(_))) {
				self.retry.insert(path);
			}
		}
	}

	/// Release this explicit request. Independent overlapping requests remain active.
	/// The plugin releases unneeded resources and strong handles at synchronization.
	pub fn unload<S: FluentScope<Catalog = C>>(&mut self) {
		self.requested.remove(&TypeId::of::<S>());
	}
}
