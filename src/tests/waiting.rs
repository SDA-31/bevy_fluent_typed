//! Consumer-level checks for native required and optional catalog resources.
use super::{OptionalCatalog, TestCatalog, bytes::pump, manifest};
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{
	FluentCatalog, FluentScope, Localization, LocalizationManifest, LocalizationPlugin, Manual,
	Module, ModuleStore, PassiveCatalog, ScopeRegistration,
};

#[derive(Resource, Default)]
struct Observations {
	required: Vec<String>,
	optional: Vec<Option<String>>,
	setups: usize,
}

#[derive(Component)]
struct Created;

#[derive(Resource)]
struct Unrelated;

fn required(catalog: Res<TestCatalog>, mut observations: ResMut<Observations>) {
	observations.required.push(catalog.0.clone());
}

fn optional(catalog: OptionalCatalog, mut observations: ResMut<Observations>) {
	observations
		.optional
		.push(catalog.map(|catalog| catalog.0.clone()));
}

fn setup(
	catalog: Res<TestCatalog>,
	mut commands: Commands,
	mut observations: ResMut<Observations>,
	mut calls: Local<usize>,
) {
	assert!(!catalog.0.is_empty());
	*calls += 1;
	assert_eq!(*calls, 1);
	observations.setups += 1;
	commands.spawn(Created);
}

fn app() -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.init_resource::<Observations>();
	app
}

#[test]
fn native_resources_wait_and_optional_observers_do_not_request_loading() {
	let mut app = app();
	// Registration order must not affect inference of the registered scope types.
	app.add_systems(Update, (required, optional))
		.add_plugins(LocalizationPlugin::<TestCatalog>::new_manual(manifest()));
	app.update();
	assert!(app.world().resource::<Observations>().required.is_empty());
	assert_eq!(app.world().resource::<Observations>().optional, [None]);
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.desired()
			.is_empty()
	);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	app.update();
	assert_eq!(app.world().resource::<Observations>().required, ["ja"]);
	assert_eq!(
		app.world().resource::<Observations>().optional[1].as_deref(),
		Some("ja")
	);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	app.update();
	assert_eq!(app.world().resource::<Observations>().required.len(), 1);
	assert_eq!(app.world().resource::<Observations>().optional[2], None);
	assert!(!app.world().contains_resource::<TestCatalog>());

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	app.update();
	assert_eq!(
		app.world().resource::<Observations>().required,
		["ja", "ja"]
	);
}

#[test]
fn full_mode_publishes_before_the_first_required_update() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog, crate::Full>::new(
		manifest(),
	))
	.add_systems(Update, required);
	app.update();
	assert_eq!(app.world().resource::<Observations>().required, ["ja"]);
}

#[test]
fn same_schedule_resource_removal_delays_a_required_consumer() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog, crate::Full>::new(
		manifest(),
	));
	app.update();
	app.add_systems(
		Update,
		(
			(|world: &mut World| {
				world.remove_resource::<TestCatalog>();
			}),
			required,
		)
			.chain(),
	);
	app.update();
	assert!(app.world().resource::<Observations>().required.is_empty());
	// PostUpdate restores the controller's still-owned snapshot.
	assert!(app.world().contains_resource::<TestCatalog>());
}

#[test]
fn deferred_setup_waits_runs_once_and_applies_commands() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new_manual(manifest()))
		.add_systems(Startup, setup);
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Observations>().setups, 0);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world.resource::<Observations>().setups == 1
	});
	assert_eq!(app.world().resource::<Observations>().setups, 1);
	assert_eq!(
		app.world_mut()
			.query::<&Created>()
			.iter(app.world())
			.count(),
		1
	);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.set_locale("es");
	app.update();
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Manual>>()
			.locale() == "es"
	});
	assert_eq!(app.world().resource::<Observations>().setups, 1);
}

#[test]
fn startup_tuple_members_wait_independently() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new_manual(manifest()))
		.add_systems(Startup, (setup, optional));
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Observations>().optional, [None]);
	assert_eq!(app.world().resource::<Observations>().setups, 0);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world.resource::<Observations>().setups == 1
	});
	assert_eq!(app.world().resource::<Observations>().setups, 1);
	assert_eq!(app.world().resource::<Observations>().optional, [None]);
}

#[test]
fn original_function_ordering_and_bevy_configuration_are_preserved() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog, crate::Full>::new(
		manifest(),
	))
	.add_systems(Update, required.run_if(|| true))
	.add_systems(
		Update,
		(|mut observations: ResMut<Observations>| observations.required.push("before".into()))
			.before(required),
	)
	.add_systems(
		Update,
		(|observations: Res<Observations>| assert_eq!(observations.required, ["before", "ja"]))
			.after(required),
	);
	app.update();
}

#[test]
fn a_locale_change_within_update_preserves_the_active_snapshot_until_publication() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog, crate::Full>::new(
		manifest(),
	));
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	app.add_systems(
		Update,
		(
			|mut localization: ResMut<Localization<TestCatalog, crate::Full>>| {
				localization.set_locale("es")
			},
			required,
		)
			.chain(),
	);
	app.update();
	assert_eq!(app.world().resource::<Observations>().required, ["ja"]);
	app.update();
	assert_eq!(
		app.world().resource::<Observations>().required,
		["ja", "es"]
	);
}

#[test]
fn ordinary_missing_resources_are_not_silenced() {
	let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		let mut app = app();
		app.add_plugins(LocalizationPlugin::<TestCatalog, crate::Full>::new(
			manifest(),
		))
		.add_systems(Update, |_: Res<TestCatalog>, _: Res<Unrelated>| {});
		app.update();
	}));
	assert!(result.is_err());
}

#[test]
fn fallible_system_outputs_retain_normal_bevy_error_handling() {
	fn fail(_: Res<TestCatalog>) -> Result {
		Err("consumer error".into())
	}

	for once in [false, true] {
		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
			let mut app = app();
			app.add_plugins(LocalizationPlugin::<TestCatalog, crate::Full>::new(
				manifest(),
			));

			if once {
				app.add_systems(Startup, fail);
			} else {
				app.add_systems(Update, fail);
			}

			for _ in 0..3 {
				app.update();
			}
		}));
		assert!(result.is_err());
	}
}

#[test]
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
fn deferred_setup_returning_an_error_is_not_repeated_by_a_nonpanicking_handler() {
	use crate::bevy::ecs::error::{FallbackErrorHandler, ignore};

	fn fail_once(_: Res<TestCatalog>, mut observations: ResMut<Observations>) -> Result {
		observations.setups += 1;
		Err("one attempted initialization".into())
	}

	let mut app = app();
	app.insert_resource(FallbackErrorHandler(ignore))
		.add_plugins(LocalizationPlugin::<TestCatalog>::new_manual(manifest()))
		.add_systems(Startup, fail_once);
	app.update();
	assert_eq!(app.world().resource::<Observations>().setups, 0);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();

	for _ in 0..3 {
		app.update();
	}

	assert_eq!(app.world().resource::<Observations>().setups, 1);
}

#[derive(Resource, Clone)]
struct OtherCatalog;

impl FluentScope for OtherCatalog {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&["other.ftl"]
	}

	fn assemble(modules: &ModuleStore<Self>) -> Option<Self> {
		modules.get::<Self>().ok().cloned()
	}
}

impl FluentCatalog for OtherCatalog {
	type Locale = &'static str;
	type Modules<'a> = &'a ModuleStore<Self>;

	fn locales() -> &'static [Self::Locale] {
		&["ja"]
	}

	fn default_locale() -> Self::Locale {
		"ja"
	}

	fn modules() -> Vec<Module<Self>> {
		vec![Module::new::<Self>("other.ftl", |_, _| Ok(Self))]
	}

	fn scopes() -> Vec<ScopeRegistration<Self>> {
		vec![ScopeRegistration::new::<Self>()]
	}

	fn view(modules: &ModuleStore<Self>) -> Self::Modules<'_> {
		modules
	}
}

#[test]
fn several_required_roots_gate_together_but_an_optional_root_does_not() {
	let mut app = app();
	let other =
		LocalizationManifest::__embedded(("ja", "ja", ".", &[("ja", "other.ftl", b"other")]));
	app.add_plugins((
		LocalizationPlugin::<TestCatalog, crate::Full>::new(manifest()),
		LocalizationPlugin::<OtherCatalog>::new_manual(other),
	))
	.add_systems(
		Update,
		(
			|_: Res<TestCatalog>, _: Res<OtherCatalog>, mut observations: ResMut<Observations>| {
				observations.required.push("both".into());
			},
			|_: Res<TestCatalog>,
			 other: Option<PassiveCatalog<OtherCatalog>>,
			 mut observations: ResMut<Observations>| {
				observations.optional.push(other.map(|_| "other".into()));
			},
		),
	);
	app.update();
	assert!(app.world().resource::<Observations>().required.is_empty());
	assert_eq!(app.world().resource::<Observations>().optional, [None]);
	app.world_mut()
		.resource_mut::<Localization<OtherCatalog, Manual>>()
		.load::<OtherCatalog>();
	app.update();
	assert_eq!(app.world().resource::<Observations>().required, ["both"]);
}
