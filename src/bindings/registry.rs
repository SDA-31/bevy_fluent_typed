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

/// Typed lifecycle bridge erased into the App-wide binding dispatcher.
/// Implementations provide one query refresh per (scope, target) binding type.
pub(crate) trait Binding: Component {
	type Scope: FluentScope;

	const SHARED_DEMAND: bool = true;

	fn synchronize_consumers(_world: &mut World) {}

	const PHASE: TargetUpdatePhase = TargetUpdatePhase::BeforeLayout;

	fn refresh(world: &mut World);

	fn unregister(world: &mut World);

	fn initialize(_world: &mut World, _entity: Entity) {}

	fn retire(_world: &mut World, _entity: Entity) {}
}

/// One binding type's callbacks, live instance count and optional automatic demand owner.
struct BindingEntry {
	root: TypeId,
	shared_demand: bool,
	synchronize_consumers: Refresh,
	scope: TypeId,
	/// Number of attached binding components of this concrete type.
	users: usize,
	refresh: Refresh,
	unregister: Refresh,
	phase: TargetUpdatePhase,
	/// Lazy demand shared by every live instance of this binding type.
	consumer: Option<Arc<Consumer>>,
	acquire: fn(&mut World) -> Arc<Consumer>,
}

/// Immutable callbacks retained while dispatch temporarily releases registry borrows.
struct ActiveBinding {
	key: TypeId,
	phase: TargetUpdatePhase,
	refresh: Refresh,
	root: TypeId,
	synchronize_consumers: Refresh,
}

/// App-wide registry of binding types and their enabled/automatic scope policies.
/// The immutable active snapshot lets callbacks mutate registrations while dispatching;
/// dirty rebuilds it before the next dispatch.
#[derive(Resource, Default)]
struct BindingRegistry {
	enabled: HashSet<TypeId>,
	automatic: HashSet<TypeId>,
	entries: HashMap<TypeId, BindingEntry>,
	active: Arc<[ActiveBinding]>,
	dirty: bool,
}

impl BindingRegistry {
	fn rebuild(&mut self) {
		self.active = self
			.entries
			.iter()
			.filter(|(_, entry)| entry.users > 0 && self.enabled.contains(&entry.scope))
			.map(|(id, entry)| ActiveBinding {
				key: *id,
				phase: entry.phase,
				refresh: entry.refresh,
				root: entry.root,
				synchronize_consumers: entry.synchronize_consumers,
			})
			.collect();
		self.dirty = false;
	}
}

/// Marker ensuring the shared before/after-layout dispatchers are installed once per App.
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
			automatic
				&& entry.shared_demand
				&& entry.scope == id
				&& entry.users > 0
				&& entry.consumer.is_none()
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
		root: TypeId::of::<<B::Scope as FluentScope>::Catalog>(),
		shared_demand: B::SHARED_DEMAND,
		synchronize_consumers: B::synchronize_consumers,
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
	let adopt = entry.shared_demand
		&& users > 0
		&& registry.automatic.contains(&scope)
		&& entry.consumer.is_none();
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
		world.init_resource::<BindingRegistry>();
		let key = TypeId::of::<B>();
		let users = {
			let mut registry = world.resource_mut::<BindingRegistry>();
			registry.entries.entry(key).or_insert_with(entry::<B>).users + 1
		};
		set_users(world, key, users);

		// Initialization can reject and remove the binding. Its removal callback
		// may run immediately, so first establish the matching registration.
		B::initialize(world, entity);
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

	for job in active.iter() {
		// Earlier adapters may remove another binding's final instance. Recheck
		// the job so its already-unregistered cache is not recreated.
		let registry = world.resource::<BindingRegistry>();
		let current = registry
			.entries
			.get(&job.key)
			.is_some_and(|entry| entry.users > 0 && registry.enabled.contains(&entry.scope));

		if job.phase == phase && current {
			(job.refresh)(world);
		}
	}
}

/// Synchronize binding selections before the provider rebuilds its demand union.
pub(crate) fn synchronize_consumers<C: crate::FluentCatalog>(world: &mut World) {
	let active = {
		let Some(mut registry) = world.get_resource_mut::<BindingRegistry>() else {
			return;
		};

		if registry.dirty {
			registry.rebuild();
		}

		registry.active.clone()
	};

	for job in active.iter() {
		if job.root == TypeId::of::<C>() {
			(job.synchronize_consumers)(world);
		}
	}
}

#[cfg(test)]
#[path = "tests/registry.rs"]
mod tests;
