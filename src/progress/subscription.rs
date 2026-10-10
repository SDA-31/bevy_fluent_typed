//! Typed resource publication erased only at the shared provider dispatcher.
use super::{LocalizationProgress, publication::Snapshot, version::Version};
use crate::bevy::prelude::*;
use crate::{FluentCatalog, FluentScope};
use std::{any::TypeId, collections::BTreeSet};

pub(super) struct Subscription<C: FluentCatalog> {
	pub(super) paths: Option<BTreeSet<&'static str>>,
	pub(super) current: fn(&World, &Version<'_, C::Locale>) -> bool,
	pub(super) publish: fn(&mut World, Snapshot<C>),
}

impl<C: FluentCatalog> Subscription<C> {
	pub(super) fn new<S: FluentScope<Catalog = C>>() -> Self {
		Self {
			paths: (TypeId::of::<S>() != TypeId::of::<C>())
				.then(|| S::module_paths().iter().copied().collect()),
			current: current::<S>,
			publish: publish::<S>,
		}
	}
}

fn current<S: FluentScope>(
	world: &World,
	version: &Version<'_, <S::Catalog as FluentCatalog>::Locale>,
) -> bool {
	world
		.get_resource::<LocalizationProgress<S>>()
		.is_some_and(|progress| progress.stamp.matches(version))
}

fn publish<S: FluentScope>(world: &mut World, next: Snapshot<S::Catalog>) {
	let next = LocalizationProgress::<S> {
		active: next.active,
		preparation: next.preparation,
		status: next.status,
		stamp: next.stamp,
	};

	if let Some(mut progress) = world.get_resource_mut::<LocalizationProgress<S>>() {
		if progress.same_visible_state(&next) {
			progress.bypass_change_detection().stamp = next.stamp;
		} else {
			*progress = next;
		}
	} else {
		world.insert_resource(next);
	}
}
