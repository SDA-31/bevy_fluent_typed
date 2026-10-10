//! Explicit subscriptions bound to one installed provider's controller mode.
use super::subscription::Subscription;
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{FluentCatalog, FluentScope, LoadingMode, LocalizationSystems};
use std::{any::TypeId, collections::BTreeMap, marker::PhantomData};

/// Installed provider identity and its concrete mode's tracker installer.
#[derive(Resource)]
pub(crate) struct Registration<C: FluentCatalog> {
	install: fn(&mut App),
	marker: PhantomData<fn() -> C>,
}

#[derive(Resource)]
pub(super) struct Requests<C: FluentCatalog> {
	pub(super) subscriptions: BTreeMap<TypeId, Subscription<C>>,
	installed: bool,
}

impl<C: FluentCatalog> Default for Requests<C> {
	fn default() -> Self {
		Self {
			subscriptions: BTreeMap::new(),
			installed: false,
		}
	}
}

pub(crate) fn register<C: FluentCatalog, M: LoadingMode>(app: &mut App) {
	app.insert_resource(Registration::<C> {
		install: install::<C, M>,
		marker: PhantomData,
	});

	if app.world().contains_resource::<Requests<C>>() {
		install::<C, M>(app);
	}
}

pub(super) fn request<S: FluentScope>(app: &mut App) {
	app.init_resource::<Requests<S::Catalog>>();
	let mut requests = app.world_mut().resource_mut::<Requests<S::Catalog>>();

	if requests.subscriptions.contains_key(&TypeId::of::<S>()) {
		return;
	}

	requests
		.subscriptions
		.insert(TypeId::of::<S>(), Subscription::new::<S>());

	if let Some(install) = app
		.world()
		.get_resource::<Registration<S::Catalog>>()
		.map(|context| context.install)
	{
		install(app);
	}
}

fn install<C: FluentCatalog, M: LoadingMode>(app: &mut App) {
	super::publication::synchronize::<C, M>(app.world_mut());
	let mut requests = app.world_mut().resource_mut::<Requests<C>>();

	if requests.installed {
		return;
	}

	requests.installed = true;
	app.add_systems(
		PreUpdate,
		super::publication::synchronize::<C, M>
			.after(LocalizationSystems::Publish)
			.in_set(LocalizationSystems::Progress),
	)
	.add_systems(
		PostUpdate,
		super::publication::synchronize::<C, M>
			.after(LocalizationSystems::Refresh)
			.in_set(LocalizationSystems::Progress),
	);
}
