use super::{
	state::State,
	tests::{Catalog, NativeText},
};
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
use crate::bevy::prelude::{Entity, Resource, World};
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{
		App, Component, DetectChanges, DetectChangesMut, EntityWorldMut, MinimalPlugins, Text,
		Text2d, TextSpan,
	},
};
use crate::bindings::Binding;
use crate::cache_policy::ProviderCachePolicy;
use crate::{CachePolicy, LocalizationTarget, LocalizationUpdate, Localized, Msg, TargetItem};
use std::{
	ops::DerefMut,
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
};

fn uncached_native<T: NativeText>(count: usize, supplied: bool) {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(ProviderCachePolicy::<Catalog>::new(CachePolicy::None));
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let message = Msg::new(move |catalog: &Catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	});
	let owners: Vec<_> = (0..count)
		.map(|_| {
			let mut owner = world.spawn(Localized::<T, Catalog>::from(message.clone()));

			if supplied {
				owner.insert(T::default());
			}

			owner.id()
		})
		.collect();
	world.flush();
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), 0);

	for &owner in &owners {
		assert!(world.get::<T>(owner).unwrap().is_empty());
		assert!(
			world
				.get::<State<Catalog, T>>(owner)
				.unwrap()
				.text
				.is_none()
		);
	}

	world.insert_resource(Catalog("ready".into()));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count);
	let tick = world
		.entity(owners[0])
		.get_ref::<T>()
		.unwrap()
		.last_changed();

	for _ in 0..3 {
		<Localized<T, Catalog> as Binding>::refresh(world);
	}

	assert_eq!(calls.load(Ordering::Relaxed), count);
	assert_eq!(
		world
			.entity(owners[0])
			.get_ref::<T>()
			.unwrap()
			.last_changed(),
		tick
	);
	world.increment_change_tick();
	world.get_mut::<T>(owners[0]).unwrap().deref_mut().clear();
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count + 1);
	assert_eq!(world.get::<T>(owners[0]).unwrap().deref(), "ready");
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(
		calls.load(Ordering::Relaxed),
		count + 1,
		"own writes must not trigger formatting"
	);
	world.entity_mut(owners[0]).insert(T::default());
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count + 2);
	assert_eq!(world.get::<T>(owners[0]).unwrap().deref(), "ready");
	world
		.entity_mut(owners[0])
		.insert(Localized::<T, Catalog>::from(message));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(
		calls.load(Ordering::Relaxed),
		count + 2,
		"cloning preserves formatter identity"
	);
	world.increment_change_tick();
	world
		.get_mut::<T>(owners[0])
		.unwrap()
		.bypass_change_detection()
		.deref_mut()
		.clear();
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert!(world.get::<T>(owners[0]).unwrap().is_empty());
	assert_eq!(calls.load(Ordering::Relaxed), count + 2);
	let observed = calls.clone();
	*world
		.get_mut::<Localized<T, Catalog>>(owners[0])
		.unwrap()
		.bypass_change_detection() = Localized::<T, Catalog>::new(move |catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		format!("{}!", catalog.0)
	});
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<T>(owners[0]).unwrap().deref(), "ready!");
	assert_eq!(calls.load(Ordering::Relaxed), count + 3);
	world.remove_resource::<Catalog>();
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert!(world.get::<T>(owners[0]).unwrap().is_empty());
	assert_eq!(calls.load(Ordering::Relaxed), count + 3);
	world.insert_resource(Catalog("next".into()));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count * 2 + 3);

	#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
	{
		use crate::bevy::ecs::entity_disabling::Disabled;

		world.entity_mut(owners[0]).insert(Disabled);
		world.insert_resource(Catalog("while disabled".into()));
		<Localized<T, Catalog> as Binding>::refresh(world);
		world.entity_mut(owners[0]).remove::<Disabled>();
		<Localized<T, Catalog> as Binding>::refresh(world);
		assert_eq!(
			world.get::<T>(owners[0]).unwrap().deref(),
			"while disabled!"
		);
	}

	world
		.entity_mut(owners[0])
		.remove::<Localized<T, Catalog>>();
	world.flush();
	assert!(world.get::<State<Catalog, T>>(owners[0]).is_none());
	<Localized<T, Catalog> as Binding>::unregister(world);
}

#[test]
fn uncached_native_targets_track_events_without_retaining_output() {
	for count in [1, 4 << 10, 32 << 10] {
		for supplied in [false, true] {
			uncached_native::<Text>(count, supplied);
			uncached_native::<Text2d>(count, supplied);
			uncached_native::<TextSpan>(count, supplied);
		}
	}
}

#[derive(Component, Default)]
struct Caption(String);

impl LocalizationTarget for Caption {
	type Data = Option<&'static mut Self>;
	type Filter = ();

	fn update(
		target: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		let Some(mut target) = target else {
			return crate::UpdateResult::Deferred;
		};

		let text = update.text();

		if target.0 != text {
			text.clone_into(&mut target.0);
		}

		crate::UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let mut target = Self::default();
		text.clone_into(&mut target.0);
		entity.insert(target);
	}
}

#[test]
fn uncached_custom_adapters_render_each_refresh_and_keep_unchanged_ticks() {
	for count in [1, 4 << 10] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let world = app.world_mut();
		world.insert_resource(ProviderCachePolicy::<Catalog>::new(CachePolicy::None));
		world.insert_resource(Catalog("ready".into()));
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = calls.clone();
		let message = Msg::new(move |catalog: &Catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		});
		let owners: Vec<_> = world
			.spawn_batch((0..count).map(|_| Localized::<Caption, Catalog>::from(message.clone())))
			.collect();
		world.flush();
		<Localized<Caption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), count);
		let tick = world
			.entity(owners[0])
			.get_ref::<Caption>()
			.unwrap()
			.last_changed();
		<Localized<Caption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), count * 2);
		assert_eq!(
			world
				.entity(owners[0])
				.get_ref::<Caption>()
				.unwrap()
				.last_changed(),
			tick
		);
		world.get_mut::<Caption>(owners[0]).unwrap().0.clear();
		world.entity_mut(owners[count - 1]).remove::<Caption>();
		<Localized<Caption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), count * 3);

		for owner in owners {
			assert_eq!(world.get::<Caption>(owner).unwrap().0, "ready");
		}

		<Localized<Caption, Catalog> as Binding>::unregister(world);
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn accessibility_uses_native_hooks_without_owning_the_neighbor_node_lifetime() {
	use crate::bevy::{a11y::AccessibilityNode, prelude::AccessibleLabel};

	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		for count in [1, 4 << 10] {
			let mut app = App::new();
			app.add_plugins(MinimalPlugins);
			let world = app.world_mut();
			world.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
			world.insert_resource(Catalog("translated".into()));
			let calls = Arc::new(AtomicUsize::new(0));
			let observed = calls.clone();
			let message = Msg::new(move |catalog: &Catalog| {
				observed.fetch_add(1, Ordering::Relaxed);
				catalog.0.clone()
			});
			let owners: Vec<_> = world
				.spawn_batch(
					(0..count)
						.map(|_| Localized::<AccessibleLabel, Catalog>::from(message.clone())),
				)
				.collect();
			world.flush();
			<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			let owner = owners[0];
			let role = world.get::<AccessibilityNode>(owner).unwrap().role();
			assert_eq!(
				world
					.get::<State<Catalog, AccessibleLabel>>(owner)
					.unwrap()
					.text
					.is_some(),
				matches!(policy, CachePolicy::Rendered)
			);
			let tick = world
				.entity(owner)
				.get_ref::<AccessibleLabel>()
				.unwrap()
				.last_changed();
			world.increment_change_tick();

			for &owner in &owners {
				let mut node = world.get_mut::<AccessibilityNode>(owner).unwrap();
				node.set_role(role);
				node.set_description("application-owned description");
				node.set_label("engine-produced name");
			}

			<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			assert_eq!(
				world
					.entity(owner)
					.get_ref::<AccessibleLabel>()
					.unwrap()
					.last_changed(),
				tick
			);

			for &owner in &owners {
				let node = world.get::<AccessibilityNode>(owner).unwrap();
				assert_eq!(node.label(), Some("engine-produced name"));
				assert_eq!(node.role(), role);
				assert_eq!(node.description(), Some("application-owned description"));
			}

			world.entity_mut(owner).remove::<AccessibilityNode>();
			<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			assert!(world.get::<AccessibilityNode>(owner).is_none());
			world
				.entity_mut(owner)
				.insert(AccessibleLabel("external".into()));
			<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
			let extra = usize::from(matches!(policy, CachePolicy::None));
			assert_eq!(calls.load(Ordering::Relaxed), count + extra);
			assert_eq!(world.get::<AccessibleLabel>(owner).unwrap().0, "translated");
			<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
			assert_eq!(
				calls.load(Ordering::Relaxed),
				count + extra,
				"own label insertion must not reformat"
			);
			world.entity_mut(owner).remove::<AccessibleLabel>();
			<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count + extra);
			world.insert_resource(Catalog("next".into()));
			<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count * 2 - 1 + extra);
			assert!(world.get::<AccessibleLabel>(owner).is_none());
			world.remove_resource::<Catalog>();
			<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
			assert!(world.get::<AccessibleLabel>(owner).is_none());
			assert_eq!(calls.load(Ordering::Relaxed), count * 2 - 1 + extra);
			<Localized<AccessibleLabel, Catalog> as Binding>::unregister(world);
		}
	}
}

#[cfg(feature = "bevy-0-20")]
fn observe_insertion<T: Component>(
	world: &mut crate::bevy::prelude::World,
	edit: fn(&mut crate::bevy::prelude::World, crate::bevy::prelude::Entity),
	previous: bool,
) {
	use crate::bevy::prelude::{Commands, Entity, Insert, Local, On, World};

	world.add_observer(
		move |event: On<Insert<T>>,
		      mut fired: Local<bool>,
		      mut first: Local<Option<Entity>>,
		      mut commands: Commands| {
			if *fired {
				return;
			}

			if previous && first.is_none() {
				*first = Some(event.entity);
				return;
			}

			*fired = true;
			let entity = first.unwrap_or(event.entity);
			commands.queue(move |world: &mut World| edit(world, entity));
		},
	);
}

#[cfg(feature = "bevy-0-19")]
fn observe_insertion<T: Component>(
	world: &mut crate::bevy::prelude::World,
	edit: fn(&mut crate::bevy::prelude::World, crate::bevy::prelude::Entity),
	previous: bool,
) {
	use crate::bevy::prelude::{Commands, Entity, Insert, Local, On, World};

	world.add_observer(
		move |event: On<Insert, T>,
		      mut fired: Local<bool>,
		      mut first: Local<Option<Entity>>,
		      mut commands: Commands| {
			if *fired {
				return;
			}

			if previous && first.is_none() {
				*first = Some(event.entity);
				return;
			}

			*fired = true;
			let entity = first.unwrap_or(event.entity);
			commands.queue(move |world: &mut World| edit(world, entity));
		},
	);
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
fn insertion_observer<T: LocalizationTarget + Component>(
	edit: fn(&mut crate::bevy::prelude::World, crate::bevy::prelude::Entity),
	read: fn(&crate::bevy::prelude::World, crate::bevy::prelude::Entity) -> &str,
) {
	for previous in [false, true] {
		for policy in [CachePolicy::Rendered, CachePolicy::None] {
			let mut app = App::new();
			app.add_plugins(MinimalPlugins);
			let world = app.world_mut();
			world.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
			world.insert_resource(Catalog("translated".into()));
			observe_insertion::<T>(world, edit, previous);
			let calls = Arc::new(AtomicUsize::new(0));
			let observed = calls.clone();
			let message = Msg::new(move |catalog: &Catalog| {
				observed.fetch_add(1, Ordering::Relaxed);
				catalog.0.clone()
			});
			let count = 1 + usize::from(previous);
			let owners: Vec<_> = world
				.spawn_batch((0..count).map(|_| Localized::<T, Catalog>::from(message.clone())))
				.collect();
			world.flush();
			<Localized<T, Catalog> as Binding>::refresh(world);
			assert_eq!(
				owners
					.iter()
					.filter(|&&owner| read(world, owner) == "observer")
					.count(),
				0
			);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			<Localized<T, Catalog> as Binding>::refresh(world);

			for &owner in &owners {
				assert_eq!(
					read(world, owner),
					"translated",
					"observer edits are external: {policy:?}, previous={previous}"
				);
			}

			let expected = count;
			assert_eq!(calls.load(Ordering::Relaxed), expected);
			<Localized<T, Catalog> as Binding>::refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), expected);
		}
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn default_initialization_observer_edits_are_translated_on_first_native_refresh() {
	insertion_observer::<Text>(
		|world, entity| {
			world.entity_mut(entity).insert(Text::new("observer"));
		},
		|world, entity| &world.get::<Text>(entity).unwrap().0,
	);
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn default_initialization_observer_edits_are_translated_on_first_semantic_refresh() {
	use crate::bevy::prelude::AccessibleLabel;

	insertion_observer::<AccessibleLabel>(
		|world, entity| {
			world
				.entity_mut(entity)
				.insert(AccessibleLabel::new("observer"));
		},
		|world, entity| &world.get::<AccessibleLabel>(entity).unwrap().0,
	);
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[derive(Resource)]
struct HookChange {
	first: Option<Entity>,
	fired: bool,
	edit: fn(&mut World, Entity),
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
fn edit_earlier_owner(
	mut world: crate::bevy::ecs::world::DeferredWorld,
	context: crate::compatibility::HookContext,
) {
	let (entity, edit) = {
		let mut change = world.resource_mut::<HookChange>();

		if change.fired {
			return;
		}

		let Some(first) = change.first else {
			change.first = Some(context.entity);
			return;
		};

		change.fired = true;
		(first, change.edit)
	};

	world
		.commands()
		.queue(move |world: &mut World| edit(world, entity));
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
fn insertion_hook<T: Component + LocalizationTarget, H: Component>(
	edit: fn(&mut World, Entity),
	read: fn(&World, Entity) -> &str,
) {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let world = app.world_mut();
		world
			.register_component_hooks::<H>()
			.on_insert(edit_earlier_owner);
		world.insert_resource(HookChange {
			first: None,
			fired: false,
			edit,
		});
		world.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
		world.insert_resource(Catalog("translated".into()));
		let message = Msg::new(|catalog: &Catalog| catalog.0.clone());
		let owners: Vec<_> = world
			.spawn_batch((0..2).map(|_| Localized::<T, Catalog>::from(message.clone())))
			.collect();
		world.flush();
		<Localized<T, Catalog> as Binding>::refresh(world);
		assert_eq!(
			owners
				.iter()
				.filter(|&&owner| read(world, owner) == "hook")
				.count(),
			0
		);
		<Localized<T, Catalog> as Binding>::refresh(world);

		for owner in owners {
			assert_eq!(
				read(world, owner),
				"translated",
				"a later owner's hook is external: {policy:?}"
			);
		}
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn default_initialization_hooks_finish_before_the_first_native_translation() {
	insertion_hook::<Text, Text>(
		|world, entity| {
			world.get_mut::<Text>(entity).unwrap().0 = "hook".into();
		},
		|world, entity| &world.get::<Text>(entity).unwrap().0,
	);
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn default_initialization_hooks_finish_before_the_first_semantic_translation() {
	use crate::bevy::{a11y::AccessibilityNode, prelude::AccessibleLabel};

	insertion_hook::<AccessibleLabel, AccessibilityNode>(
		|world, entity| {
			world
				.entity_mut(entity)
				.insert(AccessibleLabel::new("hook"));
		},
		|world, entity| &world.get::<AccessibleLabel>(entity).unwrap().0,
	);
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn node_only_removal_does_not_reinsert_the_node_or_reinvoke_the_label_formatter() {
	use crate::bevy::{a11y::AccessibilityNode, prelude::AccessibleLabel};

	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world
		.register_component_hooks::<AccessibilityNode>()
		.on_insert(edit_earlier_owner);
	world.insert_resource(HookChange {
		first: None,
		fired: true,
		edit: |world, entity| {
			world
				.entity_mut(entity)
				.insert(AccessibleLabel::new("hook"));
		},
	});
	world.insert_resource(ProviderCachePolicy::<Catalog>::new(CachePolicy::None));
	world.insert_resource(Catalog("translated".into()));
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let owner = world
		.spawn(Localized::<AccessibleLabel, Catalog>::new(move |catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		}))
		.id();
	world.flush();
	<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	let mut change = world.resource_mut::<HookChange>();
	change.first = Some(owner);
	change.fired = false;
	world.entity_mut(owner).remove::<AccessibilityNode>();
	<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	assert_eq!(world.get::<AccessibleLabel>(owner).unwrap().0, "translated");
	<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<AccessibleLabel>(owner).unwrap().0, "translated");
	assert!(world.get::<AccessibilityNode>(owner).is_none());
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	<Localized<AccessibleLabel, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), 1);
}
