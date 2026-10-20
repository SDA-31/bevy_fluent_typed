use super::tests::Catalog;
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{App, Component, Entity, EntityWorldMut, MinimalPlugins, Ref, Resource, World},
};
use crate::bindings::Binding;
use crate::cache_policy::ProviderCachePolicy;
use crate::{
	CachePolicy, LocalizationTarget, LocalizationUpdate, Localized, Msg, TargetItem,
	TargetReadItem, UpdateResult,
};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Default)]
struct Hooks {
	armed: bool,
	first: Option<Entity>,
}

#[derive(Component, Default)]
#[component(immutable, on_insert = inserted)]
struct Caption(String);

fn inserted(
	mut world: crate::bevy::ecs::world::DeferredWorld,
	event: crate::compatibility::HookContext,
) {
	let entity = {
		let mut hooks = world.resource_mut::<Hooks>();

		if !hooks.armed {
			return;
		}

		let Some(first) = hooks.first else {
			hooks.first = Some(event.entity);
			return;
		};

		hooks.armed = false;
		first
	};
	world.commands().queue(move |world: &mut World| {
		world.entity_mut(entity).insert(Caption("hook".into()));
	});
}

impl LocalizationTarget for Caption {
	type Data = Ref<'static, Self>;
	type Filter = ();

	fn needs_update(data: &TargetItem<'_, '_, Self>, update: &LocalizationUpdate<'_>) -> bool {
		update.is_changed() || update.component_changed(data)
	}

	fn update(data: TargetItem<'_, '_, Self>, update: &mut LocalizationUpdate<'_>) -> UpdateResult {
		if data.0 == update.text() {
			UpdateResult::Done
		} else {
			UpdateResult::Deferred
		}
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get::<Self>() else {
			return;
		};
		let text = update.text();

		if target.0 != text {
			entity.insert(Self(text.to_owned()));
		}
	}

	fn matches(data: TargetReadItem<'_, '_, Self>, text: &str) -> bool {
		data.0 == text
	}
}

#[derive(Component, Default)]
struct GuardedCaption(String);

impl LocalizationTarget for GuardedCaption {
	type Data = Ref<'static, Self>;
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get_ref::<Self>() else {
			return;
		};

		if !update.is_changed() && !update.component_changed(&target) {
			return;
		}

		let target = entity.get_mut::<Self>().expect("queried caption");
		update.set_text(target.map_unchanged(|target| &mut target.0));
	}

	fn matches(data: TargetReadItem<'_, '_, Self>, text: &str) -> bool {
		data.0 == text
	}
}

#[test]
fn deferred_apply_preserves_observation_for_idle_repairs_and_catalog_changes() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let world = app.world_mut();
		world.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
		world.insert_resource(Catalog("first".into()));
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = Arc::clone(&calls);
		let owner = world
			.spawn((
				GuardedCaption::default(),
				Localized::<GuardedCaption, Catalog>::new(move |catalog| {
					observed.fetch_add(1, Ordering::Relaxed);
					catalog.0.clone()
				}),
			))
			.id();
		world.flush();
		<Localized<GuardedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(world.get::<GuardedCaption>(owner).unwrap().0, "first");
		assert_eq!(calls.load(Ordering::Relaxed), 1);

		for _ in 0..3 {
			<Localized<GuardedCaption, Catalog> as Binding>::refresh(world);
		}

		assert_eq!(calls.load(Ordering::Relaxed), 1);

		world.get_mut::<GuardedCaption>(owner).unwrap().0 = "application edit".into();
		<Localized<GuardedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(world.get::<GuardedCaption>(owner).unwrap().0, "first");
		let repaired_calls = 1 + usize::from(matches!(policy, CachePolicy::None));
		assert_eq!(calls.load(Ordering::Relaxed), repaired_calls);
		<Localized<GuardedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), repaired_calls);

		world.insert_resource(Catalog("second".into()));
		<Localized<GuardedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(world.get::<GuardedCaption>(owner).unwrap().0, "second");
		assert_eq!(calls.load(Ordering::Relaxed), repaired_calls + 1);
		<Localized<GuardedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), repaired_calls + 1);
	}
}

#[test]
fn public_immutable_adapters_confirm_the_whole_batch_without_hiding_later_hooks() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let world = app.world_mut();
		world.init_resource::<Hooks>();
		world.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
		world.insert_resource(Catalog("first".into()));
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = Arc::clone(&calls);
		let message = Msg::new(move |catalog: &Catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		});
		let owners: Vec<_> = world
			.spawn_batch((0..2).map(|_| {
				(
					Caption::default(),
					Localized::<Caption, Catalog>::from(message.clone()),
				)
			}))
			.collect();
		world.flush();
		<Localized<Caption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), 2);
		world.resource_mut::<Hooks>().armed = true;
		world.insert_resource(Catalog("second".into()));
		<Localized<Caption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), 4);
		assert_eq!(
			owners
				.iter()
				.filter(|&&owner| world.get::<Caption>(owner).unwrap().0 == "hook")
				.count(),
			1
		);
		<Localized<Caption, Catalog> as Binding>::refresh(world);

		for &owner in &owners {
			assert_eq!(world.get::<Caption>(owner).unwrap().0, "second");
		}

		let expected = 4 + usize::from(matches!(policy, CachePolicy::None));
		assert_eq!(calls.load(Ordering::Relaxed), expected);
		<Localized<Caption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), expected);
	}
}

#[derive(Component, Default)]
struct Source(String);

#[derive(Component)]
struct Mirror(String);

impl LocalizationTarget for Source {
	type Data = (&'static Source, Option<&'static mut Mirror>);
	type Filter = ();

	fn update(
		(source, mirror): TargetItem<'_, '_, Self>,
		_: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		let Some(mut mirror) = mirror else {
			return UpdateResult::Deferred;
		};

		if mirror.0 != source.0 {
			source.0.clone_into(&mut mirror.0);
		}

		UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, _: &mut LocalizationUpdate<'_>) {
		let text = entity.get::<Source>().unwrap().0.clone();
		entity.insert(Mirror(text));
	}
}

#[test]
fn structural_callbacks_can_use_neighbor_data_without_requesting_a_message() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(ProviderCachePolicy::<Catalog>::new(CachePolicy::None));
	world.insert_resource(Catalog("ready".into()));
	let owner = world
		.spawn((
			Source("already available".into()),
			Localized::<Source, Catalog>::new(|_| panic!("structural work must not force Msg")),
		))
		.id();
	world.flush();
	<Localized<Source, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<Mirror>(owner).unwrap().0, "already available");
	<Localized<Source, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<Mirror>(owner).unwrap().0, "already available");
}

#[test]
fn projected_string_updates_reuse_buffers_and_keep_readback_and_change_ticks() {
	use crate::bevy::prelude::{DetectChanges, Text};

	let mut world = World::new();
	let owner = world.spawn(Text("x".into())).id();
	let mut cache = String::with_capacity(128);
	cache.push_str("previous");
	let fresh = "longer translated output".to_owned();
	let fresh_buffer = fresh.as_ptr();
	let cache_buffer = cache.as_ptr();
	let mut update = LocalizationUpdate::cached_update(true, &mut cache, Some(fresh));
	let target = world.get_mut::<Text>(owner).unwrap();
	assert!(update.set_text(target.map_unchanged(|target| &mut target.0)));
	assert_eq!(world.get::<Text>(owner).unwrap().0.as_ptr(), fresh_buffer);
	assert_eq!(update.text(), "longer translated output");
	assert_eq!(update.text(), "longer translated output");
	drop(update);
	assert_eq!(cache.as_ptr(), cache_buffer);
	let tick = world.get_mut::<Text>(owner).unwrap().last_changed();
	world.increment_change_tick();
	let mut update = LocalizationUpdate::cached_update(false, &mut cache, None);
	let target = world.get_mut::<Text>(owner).unwrap();
	assert!(!update.set_text(target.map_unchanged(|target| &mut target.0)));
	assert_eq!(world.get_mut::<Text>(owner).unwrap().last_changed(), tick);
}

#[test]
fn projected_uncached_updates_render_once_and_allow_later_text_reads() {
	use crate::bevy::prelude::Text;

	let mut world = World::new();
	let owner = world.spawn(Text::default()).id();
	let calls = AtomicUsize::new(0);
	let render = || {
		calls.fetch_add(1, Ordering::Relaxed);
		"translated".into()
	};
	let mut update = LocalizationUpdate::lazy(true, &render);
	let target = world.get_mut::<Text>(owner).unwrap();
	assert!(update.set_text(target.map_unchanged(|target| &mut target.0)));
	assert_eq!(update.text(), "translated");
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	assert_eq!(world.get::<Text>(owner).unwrap().0, "translated");
}

#[derive(Component)]
struct Enabled(bool);

#[derive(Component, Default)]
struct GatedCaption(String);

impl LocalizationTarget for GatedCaption {
	type Data = (
		Ref<'static, Enabled>,
		crate::bevy::prelude::Mut<'static, Self>,
	);
	type Filter = ();

	fn needs_update(
		(enabled, _): &TargetItem<'_, '_, Self>,
		update: &LocalizationUpdate<'_>,
	) -> bool {
		enabled.0 && update.is_changed()
	}

	fn update(
		(_, caption): TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		update.set_text(caption.map_unchanged(|caption| &mut caption.0));
		UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(caption) = entity.get_mut::<Self>() else {
			return;
		};
		update.set_text(caption.map_unchanged(|caption| &mut caption.0));
	}
}

#[test]
fn skipped_predicates_keep_catalog_and_readiness_invalidations_until_resumed() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let world = app.world_mut();
		world.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
		world.insert_resource(Catalog("first".into()));
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = Arc::clone(&calls);
		let owner = world
			.spawn((
				Enabled(true),
				GatedCaption(String::new()),
				Localized::<GatedCaption, Catalog>::new(move |catalog| {
					observed.fetch_add(1, Ordering::Relaxed);
					catalog.0.clone()
				}),
			))
			.id();
		world.flush();
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(world.get::<GatedCaption>(owner).unwrap().0, "first");
		assert_eq!(calls.load(Ordering::Relaxed), 1);
		world.get_mut::<Enabled>(owner).unwrap().0 = false;
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		world.insert_resource(Catalog("intermediate".into()));
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		world.insert_resource(Catalog("latest".into()));
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), 1);
		assert_eq!(world.get::<GatedCaption>(owner).unwrap().0, "first");
		world.get_mut::<Enabled>(owner).unwrap().0 = true;
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(world.get::<GatedCaption>(owner).unwrap().0, "latest");
		assert_eq!(calls.load(Ordering::Relaxed), 2);
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), 2);
		world.get_mut::<Enabled>(owner).unwrap().0 = false;
		world.remove_resource::<Catalog>();
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(world.get::<GatedCaption>(owner).unwrap().0, "latest");
		world.get_mut::<Enabled>(owner).unwrap().0 = true;
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		assert!(world.get::<GatedCaption>(owner).unwrap().0.is_empty());
		assert_eq!(calls.load(Ordering::Relaxed), 2);
		world.insert_resource(Catalog("ready again".into()));
		<Localized<GatedCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(world.get::<GatedCaption>(owner).unwrap().0, "ready again");
		assert_eq!(calls.load(Ordering::Relaxed), 3);
	}
}

#[derive(Default)]
struct FilterScript {
	armed: std::sync::atomic::AtomicBool,
	order: std::sync::Mutex<Vec<Entity>>,
}

#[derive(Component)]
struct ScriptHandle(Arc<FilterScript>);

#[derive(Component)]
struct Included;

#[derive(Component, Default)]
#[component(immutable)]
struct DeferredCaption(String);

impl LocalizationTarget for DeferredCaption {
	type Data = (Entity, Ref<'static, Self>, &'static ScriptHandle);
	type Filter = crate::bevy::prelude::With<Included>;

	fn needs_update(
		(_, caption, _): &TargetItem<'_, '_, Self>,
		update: &LocalizationUpdate<'_>,
	) -> bool {
		update.is_changed() || update.component_changed(caption)
	}

	fn update(
		(entity, caption, script): TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		if caption.0 == update.text() {
			return UpdateResult::Done;
		}

		if script.0.armed.load(Ordering::Relaxed) {
			script.0.order.lock().unwrap().push(entity);
		}

		UpdateResult::Deferred
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let script = Arc::clone(&entity.get::<ScriptHandle>().unwrap().0);
		let owner = entity.id();
		entity.insert(Self(update.text().to_owned()));

		if !script.armed.load(Ordering::Relaxed) {
			return;
		}

		let (restore, middle) = {
			let order = script.order.lock().unwrap();

			if owner == order[0] {
				(false, order[1])
			} else if owner == order[2] {
				(true, order[1])
			} else {
				return;
			}
		};
		entity.world_scope(|world| {
			if restore {
				world.entity_mut(middle).insert(Included);
			} else {
				world.entity_mut(middle).remove::<Included>();
			}
		});
	}

	fn matches((_, caption, _): TargetReadItem<'_, '_, Self>, text: &str) -> bool {
		caption.0 == text
	}
}

#[test]
fn a_deferred_write_skipped_by_a_transient_filter_is_retried_without_a_lost_invalidation() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let world = app.world_mut();
		world.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
		world.insert_resource(Catalog("first".into()));
		let script = Arc::new(FilterScript::default());
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = Arc::clone(&calls);
		let message = Msg::new(move |catalog: &Catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		});
		world.spawn_batch((0..3).map(|_| {
			(
				Included,
				ScriptHandle(Arc::clone(&script)),
				DeferredCaption(String::new()),
				Localized::<DeferredCaption, Catalog>::from(message.clone()),
			)
		}));
		world.flush();
		<Localized<DeferredCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), 3);
		script.armed.store(true, Ordering::Relaxed);
		world.insert_resource(Catalog("second".into()));
		<Localized<DeferredCaption, Catalog> as Binding>::refresh(world);
		let middle = script.order.lock().unwrap()[1];
		assert!(world.get::<Included>(middle).is_some());
		assert_eq!(world.get::<DeferredCaption>(middle).unwrap().0, "first");
		assert_eq!(calls.load(Ordering::Relaxed), 6);
		script.armed.store(false, Ordering::Relaxed);
		<Localized<DeferredCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(world.get::<DeferredCaption>(middle).unwrap().0, "second");
		let expected = 6 + usize::from(matches!(policy, CachePolicy::None));
		assert_eq!(calls.load(Ordering::Relaxed), expected);
		<Localized<DeferredCaption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), expected);
	}
}
