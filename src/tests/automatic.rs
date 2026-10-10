use super::{TestCatalog, bytes::pump};
use crate::bevy::{
	ecs::{self as bevy_ecs, schedule::ScheduleLabel},
	prelude::*,
};
use crate::{
	FluentScope, Localization, LocalizationAppExt, LocalizationPlugin, LocalizedText, ModuleStore,
	localized,
};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

fn app() -> (App, Arc<AtomicUsize>) {
	let reads = Arc::new(AtomicUsize::new(0));
	let observed = reads.clone();
	let plugin = LocalizationPlugin::<TestCatalog>::from_loader(move |locale, _| {
		observed.fetch_add(1, Ordering::Relaxed);
		std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	(app, reads)
}

fn binding() -> LocalizedText<TestCatalog> {
	LocalizedText::<TestCatalog>::new(|catalog| catalog.0.clone())
}

#[test]
fn unattached_messages_and_optional_parameters_do_not_start_io() {
	let (mut app, reads) = app();
	let _unattached = binding();
	let observed = Arc::new(AtomicUsize::new(0));
	let runs = observed.clone();
	app.add_localized_systems(Update, move |catalog: Option<Res<TestCatalog>>| {
		assert!(catalog.is_none());
		runs.fetch_add(1, Ordering::Relaxed);
	});

	for _ in 0..3 {
		app.update();
	}

	assert_eq!(observed.load(Ordering::Relaxed), 3);
	assert_eq!(reads.load(Ordering::Relaxed), 0);
	assert!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.desired()
			.is_empty()
	);
}

#[test]
fn inserted_bindings_share_demand_and_last_removal_releases_data() {
	let (mut app, reads) = app();
	let first = app.world_mut().spawn(binding()).id();
	let second = app.world_mut().spawn((Text2d::default(), binding())).id();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
	let weak = Arc::downgrade(
		&app.world()
			.resource::<Localization<TestCatalog>>()
			.store
			.values[&std::any::TypeId::of::<TestCatalog>()]
			.value,
	);
	app.world_mut().despawn(first);
	app.update();
	assert_eq!(app.world().get::<Text2d>(second).unwrap().0, "ja");
	assert!(weak.upgrade().is_some());
	app.world_mut()
		.entity_mut(second)
		.remove::<LocalizedText<TestCatalog>>();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert!(weak.upgrade().is_none());
	assert!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.desired()
			.is_empty()
	);
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
struct ConsumerSchedule;

#[cfg(feature = "manifest")]
#[test]
fn prepared_failures_follow_current_automatic_demand() {
	use crate::bevy::asset::{AssetLoadError, AssetLoadFailedEvent, io::AssetReaderError};
	use crate::{CatalogUpdate, CatalogUpdateReader};
	use std::sync::Mutex;

	for remove_owner in [false, true] {
		let rejections = Arc::new(Mutex::new(Vec::new()));
		let observed = rejections.clone();
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			LocalizationPlugin::<TestCatalog>::new(super::manifest()),
		))
		.add_systems(
			Update,
			move |mut updates: CatalogUpdateReader<TestCatalog>| {
				for update in updates.read() {
					if let CatalogUpdate::Rejected { locale, path, .. } = update {
						observed.lock().unwrap().push((*locale, path.clone()));
					}
				}
			},
		);
		let owner = app.world_mut().spawn(binding()).id();
		app.update();
		app.world_mut()
			.resource_mut::<Localization<TestCatalog>>()
			.set_locale("es");
		let handle = app
			.world()
			.resource::<Assets<crate::assets::PreparedModuleAsset<TestCatalog>>>()
			.reserve_handle();

		// Queue an acquisition failure for the pending target before the next
		// publication. Removing its owner must invalidate the notification too.
		{
			let mut state = app.world_mut().resource_mut::<Localization<TestCatalog>>();
			let target = state.preparation.as_mut().unwrap();
			let mut entry = crate::state::RequestedModule::new("es");
			entry.preparation_handle = Some(handle.clone());
			entry.pending = true;
			target.entries.insert("ui.ftl", entry);
			target.pending = 1;
		}

		let event = AssetLoadFailedEvent {
			id: handle.id(),
			path: "data/es/ui.ftl".into(),
			error: AssetLoadError::AssetReaderError(AssetReaderError::NotFound(
				"data/es/ui.ftl".into(),
			)),
		};
		#[cfg(feature = "bevy-0-16")]
		app.world_mut().send_event(event);
		#[cfg(not(feature = "bevy-0-16"))]
		app.world_mut().write_message(event);

		if remove_owner {
			app.world_mut().despawn(owner);
		}

		app.update();

		if remove_owner {
			assert!(rejections.lock().unwrap().is_empty());
			assert!(!app.world().contains_resource::<TestCatalog>());
		} else {
			assert_eq!(
				*rejections.lock().unwrap(),
				[(Some("es"), "ui.ftl".to_owned())]
			);
			assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
		}
	}
}

#[test]
fn registered_system_owns_demand_until_its_schedule_is_dropped() {
	let (mut app, reads) = app();
	app.add_localized_systems(ConsumerSchedule, |_: Res<TestCatalog>| {});
	app.world_mut().run_schedule(ConsumerSchedule);
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
	let schedule = app
		.world_mut()
		.resource_mut::<Schedules>()
		.remove(ConsumerSchedule)
		.unwrap();
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	drop(schedule);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn a_paused_recurring_system_keeps_ownership_and_survives_controller_replacement() {
	let (mut app, reads) = app();
	app.add_systems(
		Update,
		localized(|_: Res<TestCatalog>| panic!("paused system ran")).run_if(|| false),
	);
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.insert_resource(Localization::<TestCatalog>::new("es"));
	pump(&mut app, |world| {
		world
			.get_resource::<TestCatalog>()
			.is_some_and(|catalog| catalog.0 == "es")
	});
	assert_eq!(reads.load(Ordering::Relaxed), 2);
}

#[test]
fn once_completion_releases_its_demand() {
	let (mut app, reads) = app();
	let runs = Arc::new(AtomicUsize::new(0));
	let observed = runs.clone();
	app.add_localized_startup_systems(move |_: Res<TestCatalog>| {
		observed.fetch_add(1, Ordering::Relaxed);
	});
	pump(&mut app, |_| runs.load(Ordering::Relaxed) == 1);
	app.update();
	assert_eq!(reads.load(Ordering::Relaxed), 1);
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn once_commands_handoff_keeps_the_same_snapshot_without_another_read() {
	let (mut app, reads) = app();
	app.add_localized_startup_systems(|_: Res<TestCatalog>, mut commands: Commands| {
		let label = commands.spawn(binding()).id();
		commands.insert_resource(Spawned(label));
	});
	pump(&mut app, |world| world.contains_resource::<Spawned>());
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
	let label = app.world().resource::<Spawned>().0;
	app.world_mut().despawn(label);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[derive(Resource)]
struct Spawned(Entity);

#[derive(Resource, Clone)]
struct Unregistered;

impl FluentScope for Unregistered {
	type Catalog = TestCatalog;

	fn module_paths() -> &'static [&'static str] {
		TestCatalog::module_paths()
	}

	fn assemble(_: &ModuleStore<TestCatalog>) -> Option<Self> {
		Some(Self)
	}
}

#[test]
fn unregistered_binding_scopes_and_cancelled_prepublication_bindings_do_not_load() {
	let (mut app, reads) = app();
	let cancelled = app.world_mut().spawn(binding()).id();
	app.world_mut().despawn(cancelled);
	app.world_mut()
		.spawn(LocalizedText::<Unregistered>::new(|_| String::new()));
	app.update();
	assert_eq!(reads.load(Ordering::Relaxed), 0);
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn initial_failure_retries_only_after_explicit_reload() {
	let reads = Arc::new(AtomicUsize::new(0));
	let observed = reads.clone();
	let plugin = LocalizationPlugin::<TestCatalog>::from_loader(move |locale, _| {
		let attempt = observed.fetch_add(1, Ordering::Relaxed);
		std::future::ready(if attempt == 0 {
			Err(String::from("first read failed"))
		} else {
			Ok(locale.as_bytes().to_vec())
		})
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	let label = app.world_mut().spawn(binding()).id();
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog>>()
				.status::<TestCatalog>(),
			crate::ModuleStatus::Failed(_)
		)
	});

	for _ in 0..10 {
		app.update();
	}

	assert_eq!(reads.load(Ordering::Relaxed), 1);
	assert!(!app.world().contains_resource::<TestCatalog>());
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(crate::ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(reads.load(Ordering::Relaxed), 2);
	app.world_mut().despawn(label);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn flushed_pre_plugin_binding_is_adopted_and_released() {
	let mut app = App::new();
	let label = app.world_mut().spawn(binding()).id();
	app.world_mut().flush();
	let plugin =
		LocalizationPlugin::<TestCatalog>::from_bytes([("ja", "ui.ftl", b"Japanese".as_slice())])
			.unwrap();
	app.add_plugins((MinimalPlugins, plugin));
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "Japanese");
	app.world_mut().despawn(label);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn last_owner_cancels_pending_request_and_reacquires_fresh() {
	let reads = Arc::new(AtomicUsize::new(0));
	let observed = reads.clone();
	let plugin = LocalizationPlugin::<TestCatalog>::from_loader(move |_, _| {
		observed.fetch_add(1, Ordering::Relaxed);
		std::future::pending::<Result<Vec<u8>, String>>()
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	let first = app.world_mut().spawn(binding()).id();
	pump(&mut app, |_| reads.load(Ordering::Relaxed) == 1);
	app.world_mut().despawn(first);
	app.update();
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().pending,
		0
	);
	assert!(!app.world().contains_resource::<TestCatalog>());
	let second = app.world_mut().spawn(binding()).id();
	pump(&mut app, |_| reads.load(Ordering::Relaxed) == 2);
	app.world_mut().despawn(second);
	app.update();
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().pending,
		0
	);
}
