use super::{Binding, BindingRegistry, dispatch_phase, entry};
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{Component, Resource, World},
};
use crate::{
	FluentCatalog, FluentScope, Module, ModuleStore, ScopeRegistration, TargetUpdatePhase,
};
use std::{any::TypeId, sync::Arc};

#[derive(Resource, Clone)]
struct Catalog;

impl FluentScope for Catalog {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&[]
	}

	fn assemble(_: &ModuleStore<Self>) -> Option<Self> {
		Some(Self)
	}
}

impl FluentCatalog for Catalog {
	type Locale = &'static str;
	type Modules<'a> = ();

	fn locales() -> &'static [Self::Locale] {
		&["en"]
	}

	fn default_locale() -> Self::Locale {
		"en"
	}

	fn modules() -> Vec<Module<Self>> {
		vec![]
	}

	fn scopes() -> Vec<ScopeRegistration<Self>> {
		vec![ScopeRegistration::new::<Self>()]
	}

	fn view(_: &ModuleStore<Self>) -> Self::Modules<'_> {}
}

#[derive(Component)]
struct RemovesSibling;

#[derive(Component)]
struct RemovedSibling;

#[derive(Resource, Default)]
struct RemovedJobRuns(usize);

impl Binding for RemovesSibling {
	type Scope = Catalog;

	fn refresh(world: &mut World) {
		world
			.resource_mut::<BindingRegistry>()
			.entries
			.remove(&TypeId::of::<RemovedSibling>());
	}

	fn unregister(_: &mut World) {}
}

impl Binding for RemovedSibling {
	type Scope = Catalog;

	fn refresh(world: &mut World) {
		world.resource_mut::<RemovedJobRuns>().0 += 1;
	}

	fn unregister(_: &mut World) {}
}

#[test]
fn removed_jobs_are_not_run_from_an_earlier_dispatch_snapshot() {
	let mut world = World::new();
	world.init_resource::<BindingRegistry>();
	world.init_resource::<RemovedJobRuns>();
	let mut first = entry::<RemovesSibling>();
	first.users = 1;
	let mut second = entry::<RemovedSibling>();
	second.users = 1;
	let mut registry = world.resource_mut::<BindingRegistry>();
	registry.enabled.insert(TypeId::of::<Catalog>());
	registry
		.entries
		.insert(TypeId::of::<RemovesSibling>(), first);
	registry
		.entries
		.insert(TypeId::of::<RemovedSibling>(), second);
	registry.active = Arc::from([
		super::ActiveBinding {
			key: TypeId::of::<RemovesSibling>(),
			phase: TargetUpdatePhase::BeforeLayout,
			refresh: RemovesSibling::refresh,
			root: TypeId::of::<Catalog>(),
			synchronize_consumers: |_| {},
		},
		super::ActiveBinding {
			key: TypeId::of::<RemovedSibling>(),
			phase: TargetUpdatePhase::BeforeLayout,
			refresh: RemovedSibling::refresh,
			root: TypeId::of::<Catalog>(),
			synchronize_consumers: |_| {},
		},
	]);
	dispatch_phase(&mut world, TargetUpdatePhase::BeforeLayout);
	assert_eq!(world.resource::<RemovedJobRuns>().0, 0);
}
