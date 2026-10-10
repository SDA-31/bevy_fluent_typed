//! Cache published progress without scanning unchanged loading attempts.
use super::{LocalizationProgress, inspect};
use crate::bevy::prelude::*;
use crate::{FluentCatalog, LoadingMode, Localization};
use std::{collections::BTreeSet, sync::Arc};

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

impl<L: Copy + Eq> StoreStamp<L> {
	fn new<C: FluentCatalog<Locale = L>, M: LoadingMode>(state: &Localization<C, M>) -> Self {
		Self {
			identity: state
				.store
				.progress_version
				.identity
				.as_ref()
				.expect("tracker enabled this store")
				.clone(),
			revision: state.store.progress_version.revision,
			locale: state.locale(),
			desired: state.desired(),
			pending: state.pending,
			retry: state.retry.revision,
			handoff: handoff_pending(state),
		}
	}

	fn matches<C: FluentCatalog<Locale = L>, M: LoadingMode>(
		&self,
		state: &Localization<C, M>,
	) -> bool {
		state
			.store
			.progress_version
			.identity
			.as_ref()
			.is_some_and(|identity| Arc::ptr_eq(&self.identity, identity))
			&& self.revision == state.store.progress_version.revision
			&& self.locale == state.locale()
			&& Arc::ptr_eq(&self.desired, &state.desired)
			&& self.pending == state.pending
			&& self.retry == state.retry.revision
			&& self.handoff == handoff_pending(state)
	}
}

impl<L: Copy + Eq> Stamp<L> {
	fn new<C: FluentCatalog<Locale = L>, M: LoadingMode>(state: &Localization<C, M>) -> Self {
		Self {
			active: StoreStamp::new(state),
			preparation: state.preparation.as_deref().map(StoreStamp::new),
		}
	}

	fn matches<C: FluentCatalog<Locale = L>, M: LoadingMode>(
		&self,
		state: &Localization<C, M>,
	) -> bool {
		self.active.matches(state)
			&& match (&self.preparation, state.preparation.as_deref()) {
				(Some(stamp), Some(target)) => stamp.matches(target),
				(None, None) => true,
				_ => false,
			}
	}
}

fn handoff_pending<C: FluentCatalog, M: LoadingMode>(state: &Localization<C, M>) -> bool {
	#[cfg(feature = "manifest")]
	{
		!state.handoff_pending.is_empty()
	}

	#[cfg(not(feature = "manifest"))]
	{
		let _ = state;
		false
	}
}

fn snapshot<C: FluentCatalog, M: LoadingMode>(
	state: &Localization<C, M>,
) -> LocalizationProgress<C> {
	let preparation = state.preparation.as_deref().map(|target| {
		let store = if target.locale() == state.locale() {
			&state.store
		} else {
			&target.store
		};

		inspect(store, state.desired.iter().copied())
	});

	LocalizationProgress {
		active: inspect(&state.store, state.desired.iter().copied()),
		preparation,
		status: state.preparation_status(),
		stamp: Stamp::new(state),
	}
}

pub(crate) fn install<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	enable::<C, M>(world);
	let progress = snapshot(world.resource::<Localization<C, M>>());
	world.insert_resource(progress);
}

pub(crate) fn synchronize<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	enable::<C, M>(world);
	let state = world.resource::<Localization<C, M>>();

	if world
		.get_resource::<LocalizationProgress<C>>()
		.is_some_and(|progress| progress.stamp.matches(state))
	{
		return;
	}

	let next = snapshot(state);

	if let Some(mut progress) = world.get_resource_mut::<LocalizationProgress<C>>() {
		if progress.same_visible_state(&next) {
			progress.bypass_change_detection().stamp = next.stamp;
		} else {
			*progress = next;
		}
	} else {
		world.insert_resource(next);
	}
}

fn enable<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	let state = world.resource::<Localization<C, M>>();
	let enabled = |state: &Localization<C, M>| {
		state.store.progress_version.enabled() && state.retry.enabled()
	};

	if enabled(state) && state.preparation.as_deref().is_none_or(enabled) {
		return;
	}

	let mut state = world.resource_mut::<Localization<C, M>>();
	state.store.progress_version.enable();
	state.retry.enable();

	if let Some(target) = state.preparation.as_mut() {
		target.store.progress_version.enable();
		target.retry.enable();
	}
}
