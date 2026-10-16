use super::tests::Catalog;
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{
		App, Component, DetectChanges, DetectChangesMut, EntityWorldMut, MinimalPlugins, World,
	},
	tasks::ComputeTaskPool,
};
use crate::bindings::Binding;
use crate::{LocalizationTarget, LocalizationUpdate, Localized, Msg, TargetItem};
use std::{
	collections::HashSet,
	sync::{
		Arc, Mutex,
		atomic::{AtomicUsize, Ordering},
	},
	thread::{self, ThreadId},
};

#[derive(Default)]
struct Trace {
	updates: AtomicUsize,
	fallbacks: AtomicUsize,
	insertions: AtomicUsize,
	active: AtomicUsize,
	max_active: AtomicUsize,
	threads: Mutex<HashSet<ThreadId>>,
}

#[derive(Component)]
struct Probe {
	trace: Arc<Trace>,
	caller: ThreadId,
	busy: bool,
}

#[derive(Component, Default)]
#[component(on_add = caption_added)]
struct Caption(String);

fn caption_added(
	world: crate::bevy::ecs::world::DeferredWorld,
	context: crate::compatibility::HookContext,
) {
	let probe = world.get::<Probe>(context.entity).unwrap();
	assert_eq!(thread::current().id(), probe.caller);
	assert_eq!(probe.trace.active.load(Ordering::SeqCst), 0);
	probe.trace.insertions.fetch_add(1, Ordering::Relaxed);
}

impl LocalizationTarget for Caption {
	type Data = (&'static Probe, Option<&'static mut Self>);
	type Filter = ();

	fn update(
		(probe, caption): TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		let text = update.text();
		let trace = probe.trace.clone();
		trace.updates.fetch_add(1, Ordering::Relaxed);
		let active = trace.active.fetch_add(1, Ordering::SeqCst) + 1;
		trace.max_active.fetch_max(active, Ordering::SeqCst);

		if probe.busy {
			trace.threads.lock().unwrap().insert(thread::current().id());
			thread::yield_now();
		}

		let handled = if let Some(mut caption) = caption {
			if caption.0 != text {
				text.clone_into(&mut caption.0);
			}

			true
		} else {
			false
		};

		trace.active.fetch_sub(1, Ordering::SeqCst);
		if handled {
			crate::UpdateResult::Done
		} else {
			crate::UpdateResult::Deferred
		}
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let probe = entity.get::<Probe>().unwrap();
		assert_eq!(thread::current().id(), probe.caller);
		assert_eq!(probe.trace.active.load(Ordering::SeqCst), 0);
		probe.trace.fallbacks.fetch_add(1, Ordering::Relaxed);
		assert!(!entity.contains::<Self>());
		let mut caption = Self::default();
		text.clone_into(&mut caption.0);
		entity.insert(caption);
	}
}

fn refresh(world: &mut World) {
	<Localized<Caption, Catalog> as Binding>::refresh(world);
}

#[test]
fn custom_updates_keep_cache_repairs_and_structural_fallbacks() {
	for count in [1, 8192, 32_768] {
		for supplied in [false, true] {
			let mut app = App::new();
			app.add_plugins(MinimalPlugins);
			let world = app.world_mut();
			world.insert_resource(Catalog("first".into()));
			let trace = Arc::new(Trace::default());
			let calls = Arc::new(AtomicUsize::new(0));
			let observed = calls.clone();
			let message = Msg::new(move |catalog: &Catalog| {
				observed.fetch_add(1, Ordering::Relaxed);
				catalog.0.clone()
			});
			let owners: Vec<_> = (0..count)
				.map(|_| {
					let mut owner = world.spawn((
						Localized::<Caption, Catalog>::from(message.clone()),
						Probe {
							trace: trace.clone(),
							caller: thread::current().id(),
							busy: false,
						},
					));

					if supplied {
						owner.insert(Caption::default());
					}

					owner.id()
				})
				.collect();
			world.flush();
			refresh(world);
			assert_eq!(trace.updates.load(Ordering::Relaxed), count);
			let initial_fallbacks = if supplied { 0 } else { count };
			assert_eq!(trace.fallbacks.load(Ordering::Relaxed), initial_fallbacks);
			assert_eq!(trace.insertions.load(Ordering::Relaxed), count);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			let owner = owners[0];
			let tick = world
				.entity(owner)
				.get_ref::<Caption>()
				.unwrap()
				.last_changed();
			world.increment_change_tick();
			world.get_mut::<Caption>(owners[count - 1]).unwrap().0 = "external".into();
			refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			assert_eq!(world.get::<Caption>(owners[count - 1]).unwrap().0, "first");

			if count > 1 {
				assert_eq!(
					world
						.entity(owner)
						.get_ref::<Caption>()
						.unwrap()
						.last_changed(),
					tick
				);
			}

			world.entity_mut(owner).remove::<Caption>();
			refresh(world);
			assert_eq!(trace.insertions.load(Ordering::Relaxed), count + 1);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			assert_eq!(
				trace.fallbacks.load(Ordering::Relaxed),
				initial_fallbacks + 1
			);
			assert_eq!(world.get::<Caption>(owner).unwrap().0, "first");
			world.insert_resource(Catalog("second".into()));
			refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count * 2);

			for owner in &owners {
				assert_eq!(world.get::<Caption>(*owner).unwrap().0, "second");
			}

			world.remove_resource::<Catalog>();
			refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count * 2);
			assert!(world.get::<Caption>(owner).unwrap().0.is_empty());
			world.insert_resource(Catalog("third".into()));
			refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count * 3);

			let observed = calls.clone();
			let replacement = Msg::new(move |catalog: &Catalog| {
				observed.fetch_add(1, Ordering::Relaxed);
				format!("new {}", catalog.0)
			});

			for mut binding in world
				.query::<&mut Localized<Caption, Catalog>>()
				.iter_mut(world)
			{
				*binding.bypass_change_detection() = Localized::from(replacement.clone());
			}

			refresh(world);
			assert_eq!(calls.load(Ordering::Relaxed), count * 4);

			for owner in &owners {
				assert_eq!(world.get::<Caption>(*owner).unwrap().0, "new third");
			}

			#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
			{
				use crate::bevy::ecs::entity_disabling::Disabled;

				world.entity_mut(owner).insert(Disabled);
				refresh(world);
				world.entity_mut(owner).remove::<Disabled>();
				refresh(world);
				assert_eq!(calls.load(Ordering::Relaxed), count * 4 + 1);
			}

			let tick = world
				.entity(owner)
				.get_ref::<Caption>()
				.unwrap()
				.last_changed();
			world.increment_change_tick();
			world
				.entity_mut(owner)
				.insert(Localized::<Caption, Catalog>::new(|catalog| {
					format!("new {}", catalog.0)
				}));
			refresh(world);
			assert_eq!(
				world
					.entity(owner)
					.get_ref::<Caption>()
					.unwrap()
					.last_changed(),
				tick
			);
			<Localized<Caption, Catalog> as Binding>::unregister(world);
		}
	}
}

#[test]
fn custom_defaults_precede_catalog_readiness_and_last_removal_retires_the_cache() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	let trace = Arc::new(Trace::default());
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let lifetime = Arc::new(());
	let weak = Arc::downgrade(&lifetime);
	let message = Msg::new(move |catalog: &Catalog| {
		let _keep_alive = &lifetime;
		observed.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	});
	let owners: Vec<_> = world
		.spawn_batch((0..8192).map(|_| {
			(
				Localized::<Caption, Catalog>::from(message.clone()),
				Probe {
					trace: trace.clone(),
					caller: thread::current().id(),
					busy: false,
				},
			)
		}))
		.collect();
	drop(message);
	world.flush();
	refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), 0);
	assert_eq!(trace.insertions.load(Ordering::Relaxed), owners.len());

	for owner in &owners {
		assert!(world.get::<Caption>(*owner).unwrap().0.is_empty());
	}

	world.insert_resource(Catalog("ready".into()));
	refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), owners.len());

	for owner in owners {
		assert_eq!(world.get::<Caption>(owner).unwrap().0, "ready");
		world
			.entity_mut(owner)
			.remove::<Localized<Caption, Catalog>>();
	}

	world.flush();
	assert!(weak.upgrade().is_none());
}

#[derive(Component)]
struct Card {
	title: String,
	detail: String,
	application_value: usize,
}

struct Title;

impl LocalizationTarget for Title {
	type Data = Option<&'static mut Card>;
	type Filter = ();

	fn update(
		card: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		let Some(mut card) = card else {
			return crate::UpdateResult::Done;
		};

		let text = update.text();

		if card.title != text {
			text.clone_into(&mut card.title);
		}

		crate::UpdateResult::Done
	}

	fn apply(_: &mut EntityWorldMut<'_>, _update: &mut crate::LocalizationUpdate<'_>) {
		panic!("field adapter should wait without structural fallback");
	}
}

struct Detail;

impl LocalizationTarget for Detail {
	type Data = Option<&'static mut Card>;
	type Filter = ();

	fn update(
		card: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		let Some(mut card) = card else {
			return crate::UpdateResult::Done;
		};

		let text = update.text();

		if card.detail != text {
			text.clone_into(&mut card.detail);
		}

		crate::UpdateResult::Done
	}

	fn apply(_: &mut EntityWorldMut<'_>, _update: &mut crate::LocalizationUpdate<'_>) {
		panic!("field adapter should wait without structural fallback");
	}
}

#[test]
fn replacing_a_message_releases_its_captures_before_the_next_refresh() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("first".into()));
	let capture = Arc::new(());
	let weak = Arc::downgrade(&capture);
	let owner = world
		.spawn((
			Localized::<Title, Catalog>::new(move |catalog| {
				let _keep_alive = &capture;
				catalog.0.clone()
			}),
			Card {
				title: String::new(),
				detail: String::new(),
				application_value: 0,
			},
		))
		.id();
	world.flush();
	<Localized<Title, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<Card>(owner).unwrap().title, "first");
	world
		.entity_mut(owner)
		.insert(Localized::<Title, Catalog>::new(|_| "replacement".into()));
	assert!(weak.upgrade().is_none());
	assert_eq!(world.get::<Card>(owner).unwrap().title, "first");
	<Localized<Title, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<Card>(owner).unwrap().title, "replacement");
}

#[test]
fn parallel_field_adapters_share_a_component_without_overlapping_borrows() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("translated".into()));
	let owners: Vec<_> = world
		.spawn_batch((0..8192).map(|_| {
			(
				Localized::<Title, Catalog>::new(|catalog| format!("{} title", catalog.0)),
				Localized::<Detail, Catalog>::new(|catalog| format!("{} detail", catalog.0)),
			)
		}))
		.collect();
	world.flush();
	<Localized<Title, Catalog> as Binding>::refresh(world);
	<Localized<Detail, Catalog> as Binding>::refresh(world);

	for owner in &owners {
		assert!(!world.entity(*owner).contains::<Card>());
		world.entity_mut(*owner).insert(Card {
			title: String::new(),
			detail: String::new(),
			application_value: 42,
		});
	}

	<Localized<Title, Catalog> as Binding>::refresh(world);
	<Localized<Detail, Catalog> as Binding>::refresh(world);

	for owner in owners {
		let card = world.get::<Card>(owner).unwrap();
		assert_eq!(card.title, "translated title");
		assert_eq!(card.detail, "translated detail");
		assert_eq!(card.application_value, 42);
	}
}

#[test]
#[ignore = "requires a multi-worker pool and Bevy ECS multi_threaded; run explicitly"]
fn custom_update_callbacks_actually_overlap_on_workers() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	assert!(ComputeTaskPool::get().thread_num() > 1);
	let world = app.world_mut();
	world.insert_resource(Catalog("parallel".into()));
	let trace = Arc::new(Trace::default());
	let formatter_threads = Arc::new(Mutex::new(HashSet::new()));
	let observed = formatter_threads.clone();
	let message = Msg::new(move |catalog: &Catalog| {
		observed.lock().unwrap().insert(thread::current().id());
		thread::yield_now();
		catalog.0.clone()
	});

	world.spawn_batch((0..32_768).map(|_| {
		(
			Caption::default(),
			Localized::<Caption, Catalog>::from(message.clone()),
			Probe {
				trace: trace.clone(),
				caller: thread::current().id(),
				busy: true,
			},
		)
	}));
	world.flush();
	refresh(world);
	assert_eq!(trace.updates.load(Ordering::Relaxed), 32_768);
	assert_eq!(trace.fallbacks.load(Ordering::Relaxed), 0);
	assert!(formatter_threads.lock().unwrap().len() > 1);
	assert!(trace.threads.lock().unwrap().len() > 1);
	assert!(trace.max_active.load(Ordering::SeqCst) > 1);
	assert_eq!(trace.active.load(Ordering::SeqCst), 0);
}
