//! Mode-independent bindings read the currently published scope resource.
use crate::bevy::{
	ecs::{self as bevy_ecs, world::DeferredWorld},
	prelude::*,
};
use crate::{FluentScope, LocalizationSystems, LocalizedText, compatibility};
use std::{
	any::TypeId,
	collections::{HashMap, HashSet},
	sync::Arc,
};

type Refresh = fn(&mut World);

type WithoutTextTargets = (Without<Text>, Without<Text2d>);

struct BindingEntry {
	users: usize,
	refresh: Refresh,
}

#[derive(Resource, Default)]
struct BindingRegistry {
	enabled: HashSet<TypeId>,
	entries: HashMap<TypeId, BindingEntry>,
	active: Arc<[Refresh]>,
	dirty: bool,
}

impl BindingRegistry {
	fn rebuild(&mut self) {
		self.active = self
			.entries
			.iter()
			.filter(|(id, entry)| entry.users > 0 && self.enabled.contains(id))
			.map(|(_, entry)| entry.refresh)
			.collect();
		self.dirty = false;
	}
}

#[derive(Resource)]
struct Installed;

pub(crate) fn register<S: FluentScope>(app: &mut App) {
	if !app.world().contains_resource::<Installed>() {
		app.insert_resource(Installed)
			.init_resource::<BindingRegistry>()
			.add_systems(
				PostUpdate,
				compatibility::before_text_detection(dispatch)
					.before(compatibility::UiSystems::Content)
					.before(compatibility::update_text2d_layout)
					.in_set(LocalizationSystems::Refresh),
			);
	}

	let mut registry = app.world_mut().resource_mut::<BindingRegistry>();
	let id = TypeId::of::<S>();

	if registry.enabled.insert(id) && registry.entries.contains_key(&id) {
		registry.dirty = true;
	}
}

pub(crate) fn added<S: FluentScope>(mut world: DeferredWorld, _: compatibility::HookContext) {
	// Both hooks are deferred, including before plugin installation. Mixing an
	// immediate removal with a queued addition would leave a phantom binding.
	world.commands().queue(|world: &mut World| {
		world.init_resource::<BindingRegistry>();
		let mut registry = world.resource_mut::<BindingRegistry>();
		let entry = registry
			.entries
			.entry(TypeId::of::<S>())
			.or_insert(BindingEntry {
				users: 0,
				refresh: refresh::<S>,
			});
		entry.users += 1;

		if entry.users == 1 {
			registry.dirty = true;
		}
	});
}

pub(crate) fn removed<S: FluentScope>(mut world: DeferredWorld, _: compatibility::HookContext) {
	world.commands().queue(|world: &mut World| {
		let mut registry = world.resource_mut::<BindingRegistry>();
		let entry = registry
			.entries
			.get_mut(&TypeId::of::<S>())
			.expect("binding addition precedes removal");
		entry.users -= 1;

		if entry.users == 0 {
			registry.entries.remove(&TypeId::of::<S>());
			registry.dirty = true;
			let _ = world.unregister_system_cached(default_ui::<S>);
			let _ = world.unregister_system_cached(refresh_ui::<S>);
			let _ = world.unregister_system_cached(refresh_world::<S>);
		}
	});
}

pub(crate) fn dispatch(world: &mut World) {
	world.flush();

	if world.resource::<BindingRegistry>().dirty {
		world.resource_mut::<BindingRegistry>().rebuild();
	}

	let active = world.resource::<BindingRegistry>().active.clone();

	for refresh in active.iter() {
		refresh(world);
	}
}

fn refresh<S: FluentScope>(world: &mut World) {
	world
		.run_system_cached(default_ui::<S>)
		.expect("valid localization default UI query");
	world
		.run_system_cached(refresh_ui::<S>)
		.expect("valid localization UI query");
	world
		.run_system_cached(refresh_world::<S>)
		.expect("valid localization world-text query");
}

// Run after scene/bundle construction and deferred commands, so an explicit
// Text2d wins regardless of the order in which scene components were inserted.
fn default_ui<S: FluentScope>(
	mut commands: Commands,
	missing: Query<Entity, (With<LocalizedText<S>>, WithoutTextTargets)>,
) {
	for entity in &missing {
		commands.entity(entity).insert(Text::default());
	}
}

pub(crate) fn refresh_ui<S: FluentScope>(
	catalog: Option<Res<S>>,
	mut texts: Query<(Ref<LocalizedText<S>>, &mut Text)>,
) {
	for (binding, mut text) in &mut texts {
		let value = match &catalog {
			Some(catalog) if catalog.is_changed() || binding.is_changed() || text.is_added() => {
				binding.0.render(catalog)
			}
			Some(_) => continue,
			None => String::new(),
		};

		if text.0 != value {
			text.0 = value;
		}
	}
}

pub(crate) fn refresh_world<S: FluentScope>(
	catalog: Option<Res<S>>,
	mut texts: Query<(Ref<LocalizedText<S>>, &mut Text2d)>,
) {
	for (binding, mut text) in &mut texts {
		let value = match &catalog {
			Some(catalog) if catalog.is_changed() || binding.is_changed() || text.is_added() => {
				binding.0.render(catalog)
			}
			Some(_) => continue,
			None => String::new(),
		};

		if text.0 != value {
			text.0 = value;
		}
	}
}
