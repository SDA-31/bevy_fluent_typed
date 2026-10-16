use crate::bevy::{
	ecs as bevy_ecs,
	ecs::world::DeferredWorld,
	prelude::{App, Component, Entity, IntoScheduleConfigs, PostUpdate, Resource, World},
};
use crate::demand::{self, Consumer};
use crate::{
	FluentScope, LocalizationAccessibilitySet, LocalizationSystems, TargetUpdatePhase,
	compatibility,
};
use std::{
	any::TypeId,
	collections::{HashMap, HashSet},
	sync::Arc,
};

type Refresh = fn(&mut World);

pub(crate) trait Binding: Component {
	type Scope: FluentScope;

	const PHASE: TargetUpdatePhase = TargetUpdatePhase::BeforeLayout;

	fn refresh(world: &mut World);

	fn unregister(world: &mut World);

	fn initialize(_world: &mut World, _entity: Entity) {}

	fn retire(_world: &mut World, _entity: Entity) {}
}

struct BindingEntry {
	scope: TypeId,
	users: usize,
	refresh: Refresh,
	unregister: Refresh,
	phase: TargetUpdatePhase,
	consumer: Option<Arc<Consumer>>,
	acquire: fn(&mut World) -> Arc<Consumer>,
}

#[derive(Resource, Default)]
struct BindingRegistry {
	enabled: HashSet<TypeId>,
	automatic: HashSet<TypeId>,
	entries: HashMap<TypeId, BindingEntry>,
	active: Arc<[(TypeId, TargetUpdatePhase, Refresh)]>,
	dirty: bool,
}

impl BindingRegistry {
	fn rebuild(&mut self) {
		self.active = self
			.entries
			.iter()
			.filter(|(_, entry)| entry.users > 0 && self.enabled.contains(&entry.scope))
			.map(|(id, entry)| (*id, entry.phase, entry.refresh))
			.collect();
		self.dirty = false;
	}
}

#[derive(Resource)]
struct Installed;

fn install(app: &mut App) {
	if app.world().contains_resource::<Installed>() {
		return;
	}

	app.insert_resource(Installed)
		.init_resource::<BindingRegistry>()
		.add_systems(
			PostUpdate,
			compatibility::before_text_detection(dispatch)
				.before(compatibility::UiSystems::Content)
				.before(compatibility::update_text2d_layout)
				.in_set(LocalizationSystems::Refresh),
		)
		.add_systems(
			PostUpdate,
			compatibility::after_accessibility_layout(dispatch_after_layout)
				.in_set(LocalizationAccessibilitySet),
		);
}

pub(crate) fn register<S: FluentScope>(app: &mut App, automatic: bool) {
	install(app);
	let id = TypeId::of::<S>();
	let world = app.world_mut();
	let mut registry = world.resource_mut::<BindingRegistry>();
	registry.enabled.insert(id);
	registry.dirty = true;

	if automatic {
		registry.automatic.insert(id);
	}

	let adopt: Vec<_> = registry
		.entries
		.iter()
		.filter(|(_, entry)| {
			automatic && entry.scope == id && entry.users > 0 && entry.consumer.is_none()
		})
		.map(|(key, entry)| (*key, entry.acquire))
		.collect();

	for (key, acquire) in adopt {
		let consumer = acquire(world);
		world
			.resource_mut::<BindingRegistry>()
			.entries
			.get_mut(&key)
			.unwrap()
			.consumer = Some(consumer);
	}
}

fn entry<B: Binding>() -> BindingEntry {
	BindingEntry {
		scope: TypeId::of::<B::Scope>(),
		users: 0,
		refresh: B::refresh,
		unregister: B::unregister,
		phase: B::PHASE,
		consumer: None,
		acquire: demand::acquire::<B::Scope>,
	}
}

fn set_users(world: &mut World, key: TypeId, users: usize) {
	let mut registry = world.resource_mut::<BindingRegistry>();
	let entry = &registry.entries[&key];
	let was_active = entry.users > 0;
	let scope = entry.scope;
	let acquire = entry.acquire;
	let unregister = entry.unregister;
	let adopt = users > 0 && registry.automatic.contains(&scope) && entry.consumer.is_none();
	registry.entries.get_mut(&key).unwrap().users = users;
	registry.dirty |= was_active != (users > 0);

	if adopt {
		let consumer = acquire(world);
		world
			.resource_mut::<BindingRegistry>()
			.entries
			.get_mut(&key)
			.unwrap()
			.consumer = Some(consumer);
	}

	if was_active && users == 0 {
		world
			.resource_mut::<BindingRegistry>()
			.entries
			.get_mut(&key)
			.unwrap()
			.consumer = None;
		unregister(world);
	}
}

pub(crate) fn added<B: Binding>(mut world: DeferredWorld, context: compatibility::HookContext) {
	let entity = context.entity;

	world.commands().queue(move |world: &mut World| {
		B::initialize(world, entity);

		world.init_resource::<BindingRegistry>();
		let key = TypeId::of::<B>();
		let users = {
			let mut registry = world.resource_mut::<BindingRegistry>();
			registry.entries.entry(key).or_insert_with(entry::<B>).users + 1
		};
		set_users(world, key, users);
	});
}

pub(crate) fn removed<B: Binding>(mut world: DeferredWorld, context: compatibility::HookContext) {
	let entity = context.entity;

	world.commands().queue(move |world: &mut World| {
		B::retire(world, entity);

		let key = TypeId::of::<B>();
		let users = world.resource::<BindingRegistry>().entries[&key].users - 1;
		set_users(world, key, users);

		if users == 0 {
			world.resource_mut::<BindingRegistry>().entries.remove(&key);
		}
	});
}

pub(crate) fn dispatch(world: &mut World) {
	dispatch_phase(world, TargetUpdatePhase::BeforeLayout);
}

fn dispatch_after_layout(world: &mut World) {
	dispatch_phase(world, TargetUpdatePhase::AfterLayout);
}

fn dispatch_phase(world: &mut World, phase: TargetUpdatePhase) {
	world.flush();

	if world.resource::<BindingRegistry>().dirty {
		world.resource_mut::<BindingRegistry>().rebuild();
	}

	let active = world.resource::<BindingRegistry>().active.clone();

	for (key, target_phase, refresh) in active.iter() {
		// Earlier adapters may remove another binding's final instance. Recheck
		// the job so its already-unregistered cache is not recreated.
		let registry = world.resource::<BindingRegistry>();
		let current = registry
			.entries
			.get(key)
			.is_some_and(|entry| entry.users > 0 && registry.enabled.contains(&entry.scope));

		if *target_phase == phase && current {
			refresh(world);
		}
	}
}

#[cfg(test)]
#[path = "tests/registry.rs"]
mod tests;
