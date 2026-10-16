//! Older-backend demand, readiness and independent one-time initialization.
use super::{TestCatalog, bytes::pump};
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{Localization, LocalizationAppExt, LocalizationPlugin, Localized, Manual, localized};
use std::sync::{
	Arc, Mutex,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct Gate(Mutex<(bool, Option<std::task::Waker>)>);

impl Gate {
	fn open(&self) {
		let mut gate = self.0.lock().unwrap();
		gate.0 = true;

		if let Some(waker) = gate.1.take() {
			waker.wake();
		}
	}
}

#[derive(Resource, Default)]
struct Observed {
	reads: Vec<String>,
	optional: Vec<Option<String>>,
	setups: usize,
	post_startup: usize,
}

type Interface = TestCatalog;

fn consume(catalog: Res<Interface>, mut observed: ResMut<Observed>) {
	observed.reads.push(catalog.0.clone());
}

fn observe(catalog: Option<Res<TestCatalog>>, mut observed: ResMut<Observed>) {
	observed
		.optional
		.push(catalog.map(|catalog| catalog.0.clone()));
}

fn setup(_: Res<TestCatalog>, mut commands: Commands, mut observed: ResMut<Observed>) {
	observed.setups += 1;
	commands.spawn(Localized::<Text, TestCatalog>::new(|catalog| {
		catalog.0.clone()
	}));
}

fn app<M: crate::LoadingMode>() -> (App, Arc<Gate>, Arc<AtomicUsize>) {
	let gate = Arc::new(Gate::default());
	let allow = gate.clone();
	let reads = Arc::new(AtomicUsize::new(0));
	let requests = reads.clone();
	let plugin = LocalizationPlugin::<TestCatalog, M>::from_loader(move |locale, _| {
		requests.fetch_add(1, Ordering::Relaxed);
		let allow = allow.clone();

		async move {
			std::future::poll_fn(|context| {
				let mut gate = allow.0.lock().unwrap();

				if gate.0 {
					std::task::Poll::Ready(Ok::<_, String>(locale.as_bytes().to_vec()))
				} else {
					gate.1 = Some(context.waker().clone());
					std::task::Poll::Pending
				}
			})
			.await
		}
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin))
		.init_resource::<Observed>();
	(app, gate, reads)
}

#[test]
fn required_alias_waits_and_requests_while_optional_tuple_member_runs() {
	let (mut app, gate, reads) = app::<crate::Lazy>();
	app.add_localized_systems(Update, (consume, observe));

	for _ in 0..3 {
		app.update();
	}

	assert!(app.world().resource::<Observed>().reads.is_empty());
	assert_eq!(
		app.world().resource::<Observed>().optional,
		[None, None, None]
	);
	gate.open();
	pump(&mut app, |world| {
		!world.resource::<Observed>().reads.is_empty()
	});
	assert_eq!(app.world().resource::<Observed>().reads[0], "ja");
	assert_eq!(reads.load(Ordering::Relaxed), 1);
	assert!(
		app.world()
			.resource::<Observed>()
			.optional
			.iter()
			.any(|value| value.as_deref() == Some("ja"))
	);
}

#[test]
fn optional_only_and_unwrapped_readers_do_not_create_demand() {
	let (mut app, _, reads) = app::<crate::Lazy>();
	app.add_localized_systems(Update, observe)
		.add_systems(Update, consume.run_if(resource_exists::<TestCatalog>));

	for _ in 0..3 {
		app.update();
	}

	assert_eq!(reads.load(Ordering::Relaxed), 0);
	assert_eq!(
		app.world().resource::<Observed>().optional,
		[None, None, None]
	);
	assert!(app.world().resource::<Observed>().reads.is_empty());
}

#[test]
fn skipped_recurring_consumer_retains_demand_until_schedule_is_dropped() {
	let (mut app, gate, reads) = app::<crate::Lazy>();
	app.add_systems(Update, localized(consume).run_if(|| false));
	gate.open();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert!(app.world().resource::<Observed>().reads.is_empty());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
	let weak = Arc::downgrade(
		&app.world()
			.resource::<Localization<TestCatalog>>()
			.store
			.values[&std::any::TypeId::of::<TestCatalog>()]
			.value,
	);
	app.add_schedule(Schedule::new(Update));
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert!(weak.upgrade().is_none());
}

#[test]
fn manual_readiness_does_not_request_catalog_without_an_explicit_load() {
	let (mut app, gate, reads) = app::<Manual>();
	app.add_localized_systems(Update, consume);
	gate.open();

	for _ in 0..3 {
		app.update();
	}

	assert_eq!(reads.load(Ordering::Relaxed), 0);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		!world.resource::<Observed>().reads.is_empty()
	});
	let before = app.world().resource::<Observed>().reads.len();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(app.world().resource::<Observed>().reads.len(), before);
}

#[test]
fn independent_setup_waits_once_and_hands_demand_to_its_deferred_binding() {
	let (mut app, gate, _) = app::<crate::Lazy>();
	app.add_localized_startup_systems((setup, observe))
		.add_systems(PostStartup, |mut observed: ResMut<Observed>| {
			observed.post_startup += 1;
		});
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Observed>().setups, 0);
	assert_eq!(app.world().resource::<Observed>().optional, [None]);
	assert_eq!(app.world().resource::<Observed>().post_startup, 1);
	gate.open();
	pump(&mut app, |world| world.resource::<Observed>().setups == 1);
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	let label = app
		.world_mut()
		.query_filtered::<Entity, With<Localized<Text, TestCatalog>>>()
		.single(app.world())
		.unwrap();
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "ja");
	app.world_mut().despawn(label);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(app.world().resource::<Observed>().setups, 1);
	assert_eq!(app.world().resource::<Observed>().optional, [None]);
}

#[test]
fn localized_preserves_original_recurring_function_ordering() {
	let (mut app, gate, _) = app::<crate::Lazy>();
	app.add_systems(Update, localized(consume)).add_systems(
		Update,
		(|mut observed: ResMut<Observed>| observed.reads.push("before".into())).before(consume),
	);
	gate.open();
	pump(&mut app, |world| {
		world
			.resource::<Observed>()
			.reads
			.iter()
			.any(|value| value == "ja")
	});
	assert!(
		app.world()
			.resource::<Observed>()
			.reads
			.windows(2)
			.any(|pair| pair == ["before", "ja"])
	);
}

#[test]
fn readiness_does_not_conflict_with_unrelated_writes() {
	use crate::bevy::ecs::system::System;
	use crate::compatibility::readiness::Readiness;

	let (mut app, _, _) = app::<crate::Lazy>();
	let mut gate =
		IntoSystem::into_system(|ready: Readiness<(Res<TestCatalog>, ResMut<Observed>)>| {
			ready.ready()
		});
	let mut unrelated = IntoSystem::into_system(|_: ResMut<Observed>, _: Query<&mut Text>| {});

	#[cfg(feature = "bevy-0-16")]
	{
		gate.initialize(app.world_mut());
		unrelated.initialize(app.world_mut());
		assert!(
			gate.component_access()
				.is_compatible(unrelated.component_access())
		);
	}

	#[cfg(not(feature = "bevy-0-16"))]
	{
		let gate_access = gate.initialize(app.world_mut());
		let unrelated_access = unrelated.initialize(app.world_mut());
		assert!(
			gate_access
				.combined_access()
				.is_compatible(unrelated_access.combined_access())
		);
	}
}

#[test]
#[should_panic(expected = "startup schedules cannot retry")]
fn recurring_helper_rejects_native_startup() {
	app::<crate::Lazy>()
		.0
		.add_localized_systems(Startup, consume);
}
