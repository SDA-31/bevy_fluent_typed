//! No recurring progress work until the application explicitly opts in.
use super::TestCatalog;
use crate::bevy::prelude::*;
use crate::{
	Lazy, Localization, LocalizationAppExt, LocalizationPlugin, LocalizationProgress,
	PreparationStatus,
};
use std::{
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
	time::{Duration, Instant},
};

type Progress = LocalizationProgress<TestCatalog>;

fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if ready(app.world()) {
			return;
		}

		assert!(Instant::now() < deadline, "progress registration timed out");
		std::thread::sleep(Duration::from_millis(1));
	}
}

fn plugin() -> LocalizationPlugin<TestCatalog> {
	LocalizationPlugin::from_bytes([
		("ja", "ui.ftl", b"ja".as_slice()),
		("es", "ui.ftl", b"es".as_slice()),
	])
	.unwrap()
}

fn assert_untracked(world: &World) {
	assert!(!world.contains_resource::<Progress>());
	let state = world.resource::<Localization<TestCatalog>>();
	assert!(!state.store.progress_version.enabled());
	assert_eq!(state.store.progress_version.revision, 0);
	assert!(!state.retry.enabled());
	assert_eq!(state.retry.revision, 0);
}

#[test]
fn loading_and_passive_queries_do_not_enable_recurring_progress() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin()));
	app.finish();
	app.cleanup();
	assert_untracked(app.world());
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |world| {
		world
			.get_resource::<TestCatalog>()
			.is_some_and(|value| value.0 == "es")
	});

	for _ in 0..20 {
		app.update();
		let counts = app
			.world()
			.resource::<Localization<TestCatalog>>()
			.progress::<TestCatalog>();
		assert_eq!(counts.ready, 1);
		assert_eq!(counts.locale, "es");
		assert_untracked(app.world());
	}
}

#[test]
fn request_before_plugin_infers_lazy_mode_and_repeated_request_preserves_the_resource() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_localization_progress::<TestCatalog>();
	assert!(!app.world().contains_resource::<Progress>());
	app.add_plugins(
		LocalizationPlugin::<TestCatalog, Lazy>::from_bytes([("ja", "ui.ftl", b"ja".as_slice())])
			.unwrap(),
	);
	assert_eq!(app.world().resource::<Progress>().active().total, 0);
	let ran = Arc::new(AtomicBool::new(false));
	let observed = ran.clone();
	app.add_systems(Startup, move |progress: Res<Progress>| {
		assert_eq!(progress.active().total, 0);
		observed.store(true, Ordering::Relaxed);
	});
	app.finish();
	app.cleanup();
	app.update();
	assert!(ran.load(Ordering::Relaxed));
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world.resource::<Progress>().active().ready == 1
	});
	let tick = app
		.world()
		.get_resource_ref::<Progress>()
		.unwrap()
		.last_changed();
	app.update();
	app.add_localization_progress::<TestCatalog>();
	assert_eq!(
		app.world()
			.get_resource_ref::<Progress>()
			.unwrap()
			.last_changed(),
		tick
	);
	app.update();
	assert_eq!(
		app.world()
			.get_resource_ref::<Progress>()
			.unwrap()
			.last_changed(),
		tick
	);
}

#[test]
fn late_enable_snapshots_active_and_prepared_data_and_idle_publication_keeps_controller_ticks() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin()));
	app.finish();
	app.cleanup();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog>>()
			.preparation_status()
			== PreparationStatus::Ready
	});
	assert_untracked(app.world());
	assert!(
		!app.world()
			.resource::<Localization<TestCatalog>>()
			.preparation
			.as_ref()
			.unwrap()
			.store
			.progress_version
			.enabled()
	);
	app.add_localization_progress::<TestCatalog>();
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.active().locale, "ja");
	assert_eq!(progress.active().ready, 1);
	assert_eq!(progress.preparation().unwrap().locale, "es");
	assert_eq!(progress.preparation().unwrap().ready, 1);
	assert_eq!(progress.preparation_status(), &PreparationStatus::Ready);
	let controller_tick = app
		.world()
		.get_resource_ref::<Localization<TestCatalog>>()
		.unwrap()
		.last_changed();
	let progress_tick = app
		.world()
		.get_resource_ref::<Progress>()
		.unwrap()
		.last_changed();
	app.world_mut().increment_change_tick();
	crate::progress::publish::<TestCatalog, crate::Full>(app.world_mut());
	assert_eq!(
		app.world()
			.get_resource_ref::<Localization<TestCatalog>>()
			.unwrap()
			.last_changed(),
		controller_tick
	);
	assert_eq!(
		app.world()
			.get_resource_ref::<Progress>()
			.unwrap()
			.last_changed(),
		progress_tick
	);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(app.world().resource::<Progress>().active().locale, "es");
	assert!(app.world().resource::<Progress>().preparation().is_none());
}
