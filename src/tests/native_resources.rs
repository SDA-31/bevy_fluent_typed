use super::{TestCatalog, bytes::pump};
use crate::bevy::{
	ecs::{self as bevy_ecs, system::SystemParam},
	prelude::*,
};
use crate::{Localization, LocalizationPlugin, PassiveRes};
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

#[derive(Resource, Default)]
struct Observed(Vec<String>);

type Interface = TestCatalog;

fn consume(catalog: Res<Interface>, mut observed: ResMut<Observed>) {
	observed.0.push(catalog.0.clone());
}

#[test]
fn ordinary_native_res_alias_requests_and_reads_parsed_catalog() {
	let (mut app, reads) = app();
	app.init_resource::<Observed>().add_systems(Update, consume);
	pump(&mut app, |world| !world.resource::<Observed>().0.is_empty());
	assert_eq!(app.world().resource::<Observed>().0[0], "ja");
	assert_eq!(reads.load(Ordering::Relaxed), 1);
}

#[derive(SystemParam)]
struct Nested<'w> {
	catalog: Res<'w, Interface>,
}

#[test]
fn derived_parameters_are_discovered_without_wrapping_the_function() {
	let (mut app, reads) = app();
	app.init_resource::<Observed>()
		.add_systems(Update, |nested: Nested, mut seen: ResMut<Observed>| {
			seen.0.push(nested.catalog.0.clone())
		});
	pump(&mut app, |world| !world.resource::<Observed>().0.is_empty());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
}

#[test]
fn optional_passive_reads_run_without_loading_or_retaining_catalog() {
	let (mut app, reads) = app();
	let runs = Arc::new(AtomicUsize::new(0));
	let observed = runs.clone();
	app.add_systems(Update, move |catalog: Option<PassiveRes<TestCatalog>>| {
		assert!(catalog.is_none());
		observed.fetch_add(1, Ordering::Relaxed);
	});
	for _ in 0..3 {
		app.update();
	}
	assert_eq!(runs.load(Ordering::Relaxed), 3);
	assert_eq!(reads.load(Ordering::Relaxed), 0);
	assert!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.desired()
			.is_empty()
	);
}

#[test]
fn mixed_required_and_passive_parameters_still_own_the_required_scope() {
	let (mut app, reads) = app();
	app.init_resource::<Observed>().add_systems(
		Update,
		|catalog: Res<TestCatalog>,
		 passive: Option<PassiveRes<TestCatalog>>,
		 mut seen: ResMut<Observed>| {
			assert_eq!(passive.unwrap().0, catalog.0);
			seen.0.push(catalog.0.clone());
		},
	);
	pump(&mut app, |world| !world.resource::<Observed>().0.is_empty());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
}

#[test]
fn world_reads_and_exclusive_systems_do_not_request_all_catalogs() {
	let (mut app, reads) = app();
	app.add_systems(Update, (|_: &World| {}, |_: &mut World| {}));
	for _ in 0..3 {
		app.update();
	}
	assert_eq!(reads.load(Ordering::Relaxed), 0);
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Consumers;

#[test]
fn late_systems_acquire_and_removing_their_set_releases_catalog_memory() {
	let (mut app, reads) = app();
	app.init_resource::<Observed>();
	app.update();
	app.add_systems(Update, consume.in_set(Consumers));
	pump(&mut app, |world| !world.resource::<Observed>().0.is_empty());
	let weak = Arc::downgrade(
		&app.world()
			.resource::<Localization<TestCatalog>>()
			.store
			.values[&std::any::TypeId::of::<TestCatalog>()]
			.value,
	);
	app.add_systems(Update, (|_: Res<TestCatalog>| {}).in_set(Consumers));
	app.update();
	assert_eq!(reads.load(Ordering::Relaxed), 1);
	app.world_mut().schedule_scope(Update, |world, schedule| {
		assert_eq!(
			schedule
				.remove_systems_in_set(
					Consumers,
					world,
					crate::bevy::ecs::schedule::ScheduleCleanupPolicy::RemoveSystemsOnly
				)
				.unwrap(),
			2
		);
	});
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert!(weak.upgrade().is_none());
}

#[derive(crate::bevy::ecs::schedule::ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Custom;

#[test]
fn custom_schedules_are_prepared_by_the_first_main_update() {
	let (mut app, reads) = app();
	app.init_resource::<Observed>().add_systems(Custom, consume);
	app.update();
	assert_eq!(reads.load(Ordering::Relaxed), 0);
	app.world_mut().run_schedule(Custom);
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert!(app.world().resource::<Observed>().0.is_empty());
	app.world_mut().run_schedule(Custom);
	assert_eq!(app.world().resource::<Observed>().0, ["ja"]);
	assert_eq!(reads.load(Ordering::Relaxed), 1);
}

#[test]
fn passive_reads_allow_module_eviction_while_the_observer_stays_registered() {
	let (mut app, reads) = app();
	app.init_resource::<Observed>().add_systems(
		Update,
		|scope: Option<PassiveRes<TestCatalog>>, mut seen: ResMut<Observed>| {
			seen.0
				.push(scope.map(|scope| scope.0.clone()).unwrap_or_default());
		},
	);
	let owner = app
		.world_mut()
		.spawn(crate::LocalizedText::<TestCatalog>::new(|scope| {
			scope.0.clone()
		}))
		.id();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut().despawn(owner);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
	assert_eq!(app.world().resource::<Observed>().0.last().unwrap(), "");
}

#[test]
#[should_panic(expected = "PassiveRes conflicts")]
fn passive_parameter_does_not_hide_native_mutable_resource_conflicts() {
	let (mut app, _) = app();
	app.add_systems(
		Update,
		|_: ResMut<TestCatalog>, _: Option<PassiveRes<TestCatalog>>| {},
	);
	app.update();
}

#[derive(Resource, Clone)]
pub(super) struct FtlCatalog(Arc<crate::fluent_typed::prelude::L10nBundle>);

impl FtlCatalog {
	fn greeting(&self) -> String {
		self.0.msg("greeting", None).unwrap()
	}
}

impl crate::FluentScope for FtlCatalog {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&["hud.ftl"]
	}

	fn assemble(modules: &crate::ModuleStore<Self>) -> Option<Self> {
		modules.get::<Self>().ok().cloned()
	}
}

impl crate::FluentCatalog for FtlCatalog {
	type Locale = &'static str;
	type Modules<'a> = &'a crate::ModuleStore<Self>;

	fn locales() -> &'static [Self::Locale] {
		&["en", "fr"]
	}

	fn default_locale() -> Self::Locale {
		"en"
	}

	fn modules() -> Vec<crate::Module<Self>> {
		vec![crate::Module::new::<Self>("hud.ftl", |locale, bytes| {
			crate::fluent_typed::prelude::L10nBundle::new(locale, bytes)
				.map(|bundle| Self(Arc::new(bundle)))
				.map_err(|error| error.to_string())
		})]
	}

	fn scopes() -> Vec<crate::ScopeRegistration<Self>> {
		vec![crate::ScopeRegistration::new::<Self>()]
	}

	fn view(modules: &crate::ModuleStore<Self>) -> Self::Modules<'_> {
		modules
	}
}

#[test]
fn native_consumers_read_real_fluent_messages_and_follow_locale_switches() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<FtlCatalog>::from_bytes([
			("en", "hud.ftl", b"greeting = Hello\n".as_slice()),
			("fr", "hud.ftl", b"greeting = Bonjour\n".as_slice()),
		])
		.unwrap(),
	))
	.init_resource::<Observed>()
	.add_systems(
		Update,
		|hud: Res<FtlCatalog>, mut seen: ResMut<Observed>| seen.0.push(hud.greeting()),
	);
	pump(&mut app, |world| {
		world
			.resource::<Observed>()
			.0
			.last()
			.is_some_and(|text| text == "Hello")
	});
	app.world_mut()
		.resource_mut::<Localization<FtlCatalog>>()
		.set_locale("fr");
	pump(&mut app, |world| {
		world
			.resource::<Observed>()
			.0
			.last()
			.is_some_and(|text| text == "Bonjour")
	});
	assert!(
		app.world()
			.resource::<Observed>()
			.0
			.iter()
			.all(|text| text == "Hello" || text == "Bonjour")
	);
}

#[test]
fn plain_optional_native_resources_are_conservative_consumers() {
	let (mut app, reads) = app();
	app.init_resource::<Observed>().add_systems(
		Update,
		|scope: Option<Res<TestCatalog>>, mut seen: ResMut<Observed>| {
			seen.0.push(
				scope
					.expect("automatic native access waits for readiness")
					.0
					.clone(),
			);
		},
	);
	pump(&mut app, |world| !world.resource::<Observed>().0.is_empty());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
}

#[test]
fn consumers_in_public_localization_sets_are_not_exempt_from_discovery() {
	let (mut app, reads) = app();
	app.init_resource::<Observed>()
		.add_systems(Update, consume.in_set(crate::LocalizationSystems::Refresh));
	pump(&mut app, |world| !world.resource::<Observed>().0.is_empty());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
}

#[test]
#[should_panic(expected = "localization systems cannot move between Worlds")]
fn moving_an_initialized_schedule_to_another_world_is_rejected() {
	let (mut app, _) = app();
	app.add_systems(Custom, |_: &mut World| {});
	app.update();
	app.world_mut().run_schedule(Custom);
	let mut schedule = app
		.world_mut()
		.resource_mut::<Schedules>()
		.remove(Custom)
		.unwrap();
	let mut other_world = World::new();
	schedule.run(&mut other_world);
}
