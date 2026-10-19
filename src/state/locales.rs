//! Additional locale ownership and shared acquisition, independent of publication.
use super::{Acquisition, controller::RequestedModule};
use crate::leases::LeaseRequests;
use crate::{FluentCatalog, FluentScope, LoadingMode, Localization, ModuleLease, ModuleStore};
use std::{
	any::TypeId,
	collections::{BTreeMap, BTreeSet, HashMap},
	sync::Arc,
};

/// Loading data that can move between active, prepared and fixed-language roles.
pub(crate) struct LocaleData<C: FluentCatalog> {
	pub(crate) store: ModuleStore<C>,
	pub(crate) entries: BTreeMap<&'static str, RequestedModule<C>>,
	pub(crate) retry: crate::progress::ObservedSet,
	pub(crate) pending: usize,
	pub(crate) synchronized: u64,
	pub(crate) requests_changed: bool,
	#[cfg(feature = "manifest")]
	pub(crate) handoff_pending: BTreeSet<&'static str>,
}

impl<C: FluentCatalog> LocaleData<C> {
	pub(crate) fn empty(store: ModuleStore<C>) -> Self {
		Self {
			store,
			entries: BTreeMap::new(),
			retry: crate::progress::ObservedSet::new(),
			pending: 0,
			synchronized: u64::MAX,
			requests_changed: false,
			#[cfg(feature = "manifest")]
			handoff_pending: BTreeSet::new(),
		}
	}

	pub(crate) fn take<M: LoadingMode>(state: &mut Localization<C, M>) -> Self {
		let empty = state.store.empty_for(state.locale());

		Self {
			store: std::mem::replace(&mut state.store, empty),
			entries: std::mem::take(&mut state.entries),
			retry: std::mem::take(&mut state.retry),
			pending: std::mem::take(&mut state.pending),
			synchronized: state.synchronized,
			requests_changed: state.requests_changed,
			#[cfg(feature = "manifest")]
			handoff_pending: std::mem::take(&mut state.handoff_pending),
		}
	}

	pub(crate) fn install<M: LoadingMode>(self, state: &mut Localization<C, M>) {
		state.store = self.store;
		state.entries = self.entries;
		state.retry = self.retry;
		state.pending = self.pending;
		state.requests_changed = self.requests_changed;
		#[cfg(feature = "manifest")]
		{
			state.handoff_pending = self.handoff_pending;
		}
	}

	fn acquisition(&mut self, desired: Arc<BTreeSet<&'static str>>) -> Acquisition<'_, C> {
		Acquisition {
			store: &mut self.store,
			entries: &mut self.entries,
			retry: &mut self.retry,
			pending: &mut self.pending,
			requests_changed: &mut self.requests_changed,
			desired,
			staged: false,
			switch_when_ready: false,
			#[cfg(feature = "manifest")]
			handoff_pending: &mut self.handoff_pending,
		}
	}
}

/// Explicit fixed-language owners; data is absent while the same locale is active/prepared.
pub(crate) struct LocaleSlot<C: FluentCatalog> {
	pub(crate) locale: C::Locale,
	pub(crate) data: Option<LocaleData<C>>,
	requested: HashMap<TypeId, &'static [&'static str]>,
	leases: LeaseRequests,
	pub(crate) automatic: Arc<BTreeSet<&'static str>>,
	/// Cached union of fixed requests only.
	pub(crate) fixed: Arc<BTreeSet<&'static str>>,
	/// Physical demand, including following consumers when this slot is active/prepared.
	pub(crate) desired: Arc<BTreeSet<&'static str>>,
}

impl<C: FluentCatalog> LocaleSlot<C> {
	pub(crate) fn new(locale: C::Locale, data: Option<LocaleData<C>>) -> Self {
		Self {
			locale,
			data,
			requested: HashMap::new(),
			leases: LeaseRequests::default(),
			automatic: Arc::default(),
			fixed: Arc::default(),
			desired: Arc::default(),
		}
	}

	fn rebuild(&mut self) {
		let next = self
			.requested
			.values()
			.copied()
			.chain(self.leases.paths())
			.flat_map(|paths| paths.iter().copied())
			.chain(self.automatic.iter().copied())
			.collect();

		if *self.fixed != next {
			self.fixed = Arc::new(next);
		}
	}
}

impl<C: FluentCatalog, M: LoadingMode> Localization<C, M> {
	/// Cheap source-system guard; borrowing a controller mutably changes Bevy ticks.
	pub(crate) fn acquisition_needed(&self, assets_changed: bool) -> bool {
		let controller =
			|state: &Self| state.requests_changed || !state.retry.is_empty() || state.pending > 0;
		assets_changed
			|| controller(self)
			|| self.fixed_dirty
			|| self.has_dropped_leases()
			|| self.preparation.as_deref().is_some_and(controller)
			|| self.fixed.iter().any(|slot| {
				slot.leases.has_releases()
					|| slot.data.as_ref().is_some_and(|data| {
						data.requests_changed || !data.retry.is_empty() || data.pending > 0
					})
			})
	}

	pub(crate) fn set_fixed_automatic(&mut self, paths: Vec<Arc<BTreeSet<&'static str>>>) {
		for (slot, paths) in self.fixed.iter_mut().zip(paths) {
			if slot.automatic == paths {
				continue;
			}

			slot.automatic = paths;
			slot.rebuild();
			self.fixed_dirty = true;
			self.views_changed = true;
		}
	}

	/// Active physical demand, including fixed owners of the active language.
	pub(crate) fn progress_desired(&self) -> &Arc<BTreeSet<&'static str>> {
		self.fixed
			.iter()
			.find(|slot| slot.locale == self.locale() && !slot.fixed.is_empty())
			.map_or(&self.desired, |slot| &slot.desired)
	}

	pub(crate) fn locale_index(&self, locale: C::Locale) -> usize {
		self.fixed
			.iter()
			.position(|slot| slot.locale == locale)
			.expect("locale must belong to the provider")
	}

	pub(crate) fn locale_store(&self, locale: C::Locale) -> &ModuleStore<C> {
		if self.locale() == locale {
			return &self.store;
		}

		if let Some(target) = self
			.preparation
			.as_deref()
			.filter(|target| target.locale() == locale)
		{
			return &target.store;
		}

		&self.fixed[self.locale_index(locale)]
			.data
			.as_ref()
			.expect("inactive locale data")
			.store
	}

	pub(crate) fn take_locale(&mut self, locale: C::Locale) -> Option<LocaleData<C>> {
		let index = self.locale_index(locale);
		self.fixed_dirty = true;
		self.views_changed = true;
		self.fixed[index].data.take()
	}

	pub(crate) fn retain_locale(&mut self, mut data: LocaleData<C>) {
		let index = self.locale_index(data.store.locale());
		let desired = self.fixed[index].fixed.clone();
		crate::loading::release_state(&mut data.acquisition(desired));
		crate::resources::assemble_store(&mut data.store);
		self.fixed[index].data = Some(data);
		self.fixed_dirty = true;
		self.views_changed = true;
	}

	pub(crate) fn fixed_load<S: FluentScope<Catalog = C>>(&mut self, index: usize) {
		self.fixed[index]
			.requested
			.insert(TypeId::of::<S>(), S::module_paths());
		self.fixed[index].rebuild();
		self.fixed_dirty = true;
		self.views_changed = true;
		let locale = self.fixed[index].locale;
		self.visit_acquisitions(|state| {
			if state.locale() != locale {
				return;
			}

			for &path in S::module_paths() {
				if matches!(
					state.store.states.get(path),
					Some(crate::ModuleStatus::Failed(_))
				) {
					state.retry.insert(path);
				}
			}
		});
	}

	pub(crate) fn fixed_unload<S: FluentScope<Catalog = C>>(&mut self, index: usize) {
		self.fixed[index].requested.remove(&TypeId::of::<S>());
		self.fixed[index].rebuild();
		self.fixed_dirty = true;
		self.views_changed = true;
	}

	pub(crate) fn fixed_hold<S: FluentScope<Catalog = C>>(
		&mut self,
		index: usize,
	) -> ModuleLease<S> {
		let token = self.fixed[index].leases.hold::<S>();
		self.fixed[index].rebuild();
		self.fixed_dirty = true;
		self.views_changed = true;
		token
	}

	/// Rebuild unions only after demand or role changes; Idle retains the shared sets.
	fn synchronize_fixed(&mut self) {
		for slot in &mut self.fixed {
			if slot.leases.has_releases() && slot.leases.release_dropped() {
				slot.rebuild();
				self.fixed_dirty = true;
				self.views_changed = true;
			}
		}

		if !self.fixed_dirty && !self.requests_changed {
			return;
		}

		let active = self.locale();
		let prepared = self.prepared_locale();

		for slot in &mut self.fixed {
			let following = slot.locale == active || prepared == Some(slot.locale);
			let next = if following && !slot.fixed.is_empty() {
				Arc::new(
					self.desired
						.iter()
						.chain(slot.fixed.iter())
						.copied()
						.collect(),
				)
			} else if following {
				self.desired.clone()
			} else {
				slot.fixed.clone()
			};

			if slot.desired == next {
				continue;
			}

			slot.desired = next;

			if slot.locale == active {
				self.requests_changed = true;
			}

			if let Some(target) = self
				.preparation
				.as_mut()
				.filter(|target| target.locale() == slot.locale)
			{
				target.requests_changed = true;
			}

			if let Some(data) = &mut slot.data {
				data.requests_changed = true;
			}
		}

		self.fixed_dirty = false;
	}

	/// Visit each physical locale once, sharing one acquisition implementation per source.
	pub(crate) fn visit_acquisitions(&mut self, mut visit: impl FnMut(&mut Acquisition<'_, C>)) {
		self.synchronize_fixed();

		if self.fixed.is_empty() {
			visit(&mut Acquisition::controller(self));
			return;
		}

		let active = self.locale_index(self.locale());
		let desired = self.fixed[active].desired.clone();
		let mut state = Acquisition::controller(self);
		state.desired = desired;
		visit(&mut state);

		let target_index = self
			.prepared_locale()
			.map(|locale| self.locale_index(locale));

		if let Some(index) = target_index.filter(|index| *index != active) {
			let desired = self.fixed[index].desired.clone();
			let target = self.preparation.as_mut().expect("prepared locale");
			let mut state = Acquisition::controller(target);
			state.desired = desired;
			visit(&mut state);
		}

		for slot in &mut self.fixed {
			let Some(data) = &mut slot.data else {
				continue;
			};
			visit(&mut data.acquisition(slot.desired.clone()));
		}
	}
}
