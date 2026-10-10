//! One dispatcher per provider, with independent cached scope resources.
use super::{
	LoadingProgress, inspect,
	registration::Requests,
	version::{Stamp, Version},
};
use crate::bevy::prelude::*;
use crate::{FluentCatalog, LoadingMode, Localization, PreparationStatus};
use std::collections::BTreeSet;

pub(super) struct Snapshot<C: FluentCatalog> {
	pub(super) active: LoadingProgress<C::Locale>,
	pub(super) preparation: Option<LoadingProgress<C::Locale>>,
	pub(super) status: PreparationStatus,
	pub(super) stamp: Stamp<C::Locale>,
}

fn snapshot<C: FluentCatalog, M: LoadingMode>(
	state: &Localization<C, M>,
	selected: Option<&BTreeSet<&'static str>>,
) -> Snapshot<C> {
	let paths = selected.unwrap_or(&state.desired);
	let preparation = state.preparation.as_deref().map(|target| {
		let store = if target.locale() == state.locale() {
			&state.store
		} else {
			&target.store
		};

		inspect(store, paths.iter().copied())
	});

	Snapshot {
		active: inspect(&state.store, paths.iter().copied()),
		preparation,
		status: state.preparation_status(),
		stamp: Stamp::new(&Version::new(state)),
	}
}

pub(crate) fn synchronize<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	enable::<C, M>(world);
	world.resource_scope(|world, requests: Mut<Requests<C>>| {
		for subscription in requests.subscriptions.values() {
			let state = world.resource::<Localization<C, M>>();

			if (subscription.current)(world, &Version::new(state)) {
				continue;
			}

			let next = snapshot(state, subscription.paths.as_ref());
			(subscription.publish)(world, next);
		}
	});
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
