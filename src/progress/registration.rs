//! Explicit tracker registration, bound to the provider's installed controller mode.
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{FluentCatalog, LoadingMode, LocalizationSystems};
use std::marker::PhantomData;

/// Installed provider identity and its concrete mode's tracker installer.
#[derive(Resource)]
pub(crate) struct Registration<C: FluentCatalog> {
	install: fn(&mut App),
	marker: PhantomData<fn() -> C>,
}

#[derive(Resource)]
struct Requested<C: FluentCatalog>(PhantomData<fn() -> C>);

pub(crate) fn register<C: FluentCatalog, M: LoadingMode>(app: &mut App) {
	app.insert_resource(Registration::<C> {
		install: install::<C, M>,
		marker: PhantomData,
	});

	if app.world().contains_resource::<Requested<C>>() {
		install::<C, M>(app);
	}
}

pub(crate) fn request<C: FluentCatalog>(app: &mut App) {
	if app.world().contains_resource::<Requested<C>>() {
		return;
	}

	app.insert_resource(Requested::<C>(PhantomData));

	if let Some(install) = app
		.world()
		.get_resource::<Registration<C>>()
		.map(|context| context.install)
	{
		install(app);
	}
}

fn install<C: FluentCatalog, M: LoadingMode>(app: &mut App) {
	super::publication::install::<C, M>(app.world_mut());
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
