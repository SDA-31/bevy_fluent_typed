//! Independent ownership without changing idempotent explicit requests.
use super::TestCatalog;
use crate::bevy::prelude::*;
use crate::{
	Localization, LocalizationPlugin, LocalizedText, Manual, ModuleStatus, ReloadCatalogs,
};
use std::{
	sync::{
		Arc, Barrier,
		atomic::{AtomicUsize, Ordering},
	},
	time::{Duration, Instant},
};

fn app(plugin: LocalizationPlugin<TestCatalog, Manual>) -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();
	app
}

fn bytes_app() -> App {
	app(LocalizationPlugin::from_bytes([
		("ja", "ui.ftl", b"Japanese".as_slice()),
		("es", "ui.ftl", b"Spanish".as_slice()),
	])
	.unwrap())
}

fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if ready(app.world()) {
			return;
		}

		assert!(Instant::now() < deadline, "lease publication timed out");
		std::thread::sleep(Duration::from_millis(1));
	}
}

#[test]
fn identical_leases_are_independent_and_release_at_publication() {
	let mut app = bytes_app();
	let (first, second) = {
		let mut state = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Manual>>();
		(state.hold::<TestCatalog>(), state.hold::<TestCatalog>())
	};

	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	let tick = app
		.world()
		.get_resource_ref::<TestCatalog>()
		.unwrap()
		.last_changed();
	drop(first);
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "Japanese");
	assert_eq!(
		app.world()
			.get_resource_ref::<TestCatalog>()
			.unwrap()
			.last_changed(),
		tick
	);
	let state_tick = app
		.world()
		.get_resource_ref::<Localization<TestCatalog, Manual>>()
		.unwrap()
		.last_changed();
	app.update();
	assert_eq!(
		app.world()
			.get_resource_ref::<Localization<TestCatalog, Manual>>()
			.unwrap()
			.last_changed(),
		state_tick
	);
	drop(second);
	assert!(app.world().contains_resource::<TestCatalog>());
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.status::<TestCatalog>(),
		ModuleStatus::Unloaded
	);
}

#[test]
fn explicit_load_and_unload_never_release_a_lease() {
	let mut app = bytes_app();
	let lease = {
		let mut state = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Manual>>();
		state.load::<TestCatalog>();
		state.load::<TestCatalog>();
		state.hold::<TestCatalog>()
	};

	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	drop(lease);
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn cross_thread_drop_after_locale_change_releases_the_same_owner() {
	let mut app = bytes_app();
	let lease = app
		.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.hold::<TestCatalog>();

	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.set_locale("es");
	pump(&mut app, |world| {
		world
			.get_resource::<TestCatalog>()
			.is_some_and(|catalog| catalog.0 == "Spanish")
	});
	std::thread::spawn(move || drop(lease)).join().unwrap();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn dropping_before_first_publication_performs_no_io_or_implicit_binding_load() {
	let reads = Arc::new(AtomicUsize::new(0));
	let observed = reads.clone();
	let mut app = app(LocalizationPlugin::from_loader(move |_, _| {
		observed.fetch_add(1, Ordering::Relaxed);
		std::future::ready(Ok::<_, String>(b"data".to_vec()))
	}));
	app.world_mut()
		.spawn(LocalizedText::<TestCatalog>::new(|catalog| {
			catalog.0.clone()
		}));
	let lease = app
		.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.hold::<TestCatalog>();
	drop(lease);
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(ReloadCatalogs::<TestCatalog>::default());

	for _ in 0..3 {
		app.update();
	}

	assert_eq!(reads.load(Ordering::Relaxed), 0);
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.retry
			.is_empty()
	);
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.desired()
			.is_empty()
	);
}

#[test]
fn despawning_a_lease_component_releases_demand_while_arc_sharing_waits_for_last_drop() {
	let mut app = bytes_app();
	let lease = app
		.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.hold::<TestCatalog>();
	let entity = app.world_mut().spawn(lease).id();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	let mut owner = Some(entity);

	app.add_systems(Update, move |mut commands: Commands| {
		let Some(entity) = owner.take() else {
			return;
		};

		commands.entity(entity).despawn();
	});

	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	let lease = Arc::new(
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Manual>>()
			.hold::<TestCatalog>(),
	);
	let shared = lease.clone();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	drop(lease);
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	drop(shared);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn old_controller_tokens_cannot_release_replacement_controller_demand() {
	let mut state = Localization::<TestCatalog, Manual>::default();
	let old = state.hold::<TestCatalog>();
	drop(state);
	let mut replacement = Localization::<TestCatalog, Manual>::default();
	let new = replacement.hold::<TestCatalog>();
	std::thread::spawn(move || drop(old)).join().unwrap();
	assert!(!replacement.has_dropped_leases());
	assert!(replacement.desired().contains("ui.ftl"));
	drop(new);
	assert!(replacement.has_dropped_leases());
	replacement.release_dropped_leases();
	assert!(replacement.desired().is_empty());
}

#[test]
fn held_failed_module_retries_through_load_without_losing_the_lease() {
	let attempts = Arc::new(AtomicUsize::new(0));
	let observed = attempts.clone();
	let mut app = app(LocalizationPlugin::from_loader(move |_, _| {
		let attempt = observed.fetch_add(1, Ordering::Relaxed);
		std::future::ready(if attempt == 0 {
			Err("offline".to_owned())
		} else {
			Ok(b"recovered".to_vec())
		})
	}));
	let first = app
		.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.hold::<TestCatalog>();

	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog, Manual>>()
				.status::<TestCatalog>(),
			ModuleStatus::Failed(_)
		)
	});
	let second = app
		.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.hold::<TestCatalog>();
	app.update();
	assert_eq!(attempts.load(Ordering::Relaxed), 1);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	drop(first);
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "recovered");
	drop(second);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(attempts.load(Ordering::Relaxed), 2);
}

#[cfg(feature = "manifest")]
#[test]
fn embedded_manifest_publication_also_drains_lease_drops() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<TestCatalog, Manual>::new(super::manifest()),
	));
	let lease = app
		.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.hold::<TestCatalog>();
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	drop(lease);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn concurrent_drops_during_publication_never_release_a_surviving_owner() {
	let mut state = Localization::<TestCatalog, Manual>::default();
	let survivor = state.hold::<TestCatalog>();
	let owners: Vec<_> = (0..4)
		.map(|_| {
			(0..32)
				.map(|_| state.hold::<TestCatalog>())
				.collect::<Vec<_>>()
		})
		.collect();
	let start = Arc::new(Barrier::new(5));
	let finished = Arc::new(AtomicUsize::new(0));

	std::thread::scope(|threads| {
		for owners in owners {
			let start = start.clone();
			let finished = finished.clone();

			threads.spawn(move || {
				start.wait();

				for owner in owners {
					drop(owner);
					std::thread::yield_now();
				}

				finished.fetch_add(1, Ordering::Release);
			});
		}

		start.wait();
		let deadline = Instant::now() + Duration::from_secs(10);

		while finished.load(Ordering::Acquire) != 4 {
			if state.has_dropped_leases() {
				state.release_dropped_leases();
			}

			assert!(state.desired().contains("ui.ftl"));
			assert!(
				Instant::now() < deadline,
				"concurrent lease release timed out"
			);
			std::thread::yield_now();
		}
	});

	state.release_dropped_leases();
	assert!(state.desired().contains("ui.ftl"));
	drop(survivor);
	state.release_dropped_leases();
	assert!(state.desired().is_empty());
}
