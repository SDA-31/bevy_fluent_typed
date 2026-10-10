//! Borrowed controller versions and owned cache stamps shared by typed views.
use super::tracking::StoreVersion;
use crate::{FluentCatalog, LoadingMode, Localization};
use std::{collections::BTreeSet, sync::Arc};

pub(super) struct Version<'a, L> {
	active: StoreView<'a, L>,
	preparation: Option<StoreView<'a, L>>,
}

struct StoreView<'a, L> {
	store: &'a StoreVersion,
	locale: L,
	desired: &'a Arc<BTreeSet<&'static str>>,
	pending: usize,
	retry: u64,
	handoff: bool,
}

impl<'a, L: Copy + Eq> Version<'a, L> {
	pub(super) fn new<C: FluentCatalog<Locale = L>, M: LoadingMode>(
		state: &'a Localization<C, M>,
	) -> Self {
		Self {
			active: StoreView::new(state),
			preparation: state.preparation.as_deref().map(StoreView::new),
		}
	}
}

impl<'a, L: Copy + Eq> StoreView<'a, L> {
	fn new<C: FluentCatalog<Locale = L>, M: LoadingMode>(state: &'a Localization<C, M>) -> Self {
		#[cfg(feature = "manifest")]
		let handoff = !state.handoff_pending.is_empty();
		#[cfg(not(feature = "manifest"))]
		let handoff = false;

		Self {
			store: &state.store.progress_version,
			locale: state.locale(),
			desired: &state.desired,
			pending: state.pending,
			retry: state.retry.revision,
			handoff,
		}
	}
}

pub(super) struct Stamp<L> {
	active: StoreStamp<L>,
	preparation: Option<StoreStamp<L>>,
}

struct StoreStamp<L> {
	identity: Arc<()>,
	revision: u64,
	locale: L,
	desired: Arc<BTreeSet<&'static str>>,
	pending: usize,
	retry: u64,
	handoff: bool,
}

impl<L: Copy + Eq> Stamp<L> {
	pub(super) fn new(version: &Version<'_, L>) -> Self {
		Self {
			active: StoreStamp::new(&version.active),
			preparation: version.preparation.as_ref().map(StoreStamp::new),
		}
	}

	pub(super) fn matches(&self, version: &Version<'_, L>) -> bool {
		self.active.matches(&version.active)
			&& match (&self.preparation, &version.preparation) {
				(Some(stamp), Some(target)) => stamp.matches(target),
				(None, None) => true,
				_ => false,
			}
	}
}

impl<L: Copy + Eq> StoreStamp<L> {
	fn new(view: &StoreView<'_, L>) -> Self {
		Self {
			identity: view
				.store
				.identity
				.as_ref()
				.expect("tracker enabled this store")
				.clone(),
			revision: view.store.revision,
			locale: view.locale,
			desired: view.desired.clone(),
			pending: view.pending,
			retry: view.retry,
			handoff: view.handoff,
		}
	}

	fn matches(&self, view: &StoreView<'_, L>) -> bool {
		view.store
			.identity
			.as_ref()
			.is_some_and(|identity| Arc::ptr_eq(&self.identity, identity))
			&& self.revision == view.store.revision
			&& self.locale == view.locale
			&& Arc::ptr_eq(&self.desired, view.desired)
			&& self.pending == view.pending
			&& self.retry == view.retry
			&& self.handoff == view.handoff
	}
}
