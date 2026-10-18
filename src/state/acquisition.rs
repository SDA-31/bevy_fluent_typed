//! Borrowed acquisition state shared by active, staged and additional locales.
use super::controller::RequestedModule;
use crate::{FluentCatalog, LoadingMode, Localization, ModuleStore};
use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};

/// One language's mutable loading state, without provider selection or ECS publication.
pub(crate) struct Acquisition<'a, C: FluentCatalog> {
	pub(crate) store: &'a mut ModuleStore<C>,
	pub(crate) entries: &'a mut BTreeMap<&'static str, RequestedModule<C>>,
	pub(crate) retry: &'a mut crate::progress::ObservedSet,
	pub(crate) pending: &'a mut usize,
	pub(crate) requests_changed: &'a mut bool,
	pub(crate) desired: Arc<BTreeSet<&'static str>>,
	pub(crate) staged: bool,
	pub(crate) switch_when_ready: bool,
	#[cfg(feature = "manifest")]
	pub(crate) handoff_pending: &'a mut BTreeSet<&'static str>,
}

impl<'a, C: FluentCatalog> Acquisition<'a, C> {
	pub(crate) fn controller<M: LoadingMode>(state: &'a mut Localization<C, M>) -> Self {
		Self {
			desired: state.desired(),
			store: &mut state.store,
			entries: &mut state.entries,
			retry: &mut state.retry,
			pending: &mut state.pending,
			requests_changed: &mut state.requests_changed,
			staged: state.staged,
			switch_when_ready: state.switch_when_ready,
			#[cfg(feature = "manifest")]
			handoff_pending: &mut state.handoff_pending,
		}
	}

	pub(crate) fn locale(&self) -> C::Locale {
		self.store.locale()
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
			*self.pending -= 1;
		}

		entry
	}
}
