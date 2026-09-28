//! Consumer-level checks for native required and optional catalog resources.
use super::{TestCatalog, manifest};
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{
	FluentCatalog, FluentScope, Lazy, Localization, LocalizationAppExt, LocalizationManifest,
	LocalizationPlugin, Module, ModuleStore, ScopeRegistration, localized,
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

fn optional(catalog: Option<Res<TestCatalog>>, mut observations: ResMut<Observations>) {
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
	app.add_localized_systems(Update, (required, optional))
		.add_plugins(LocalizationPlugin::<TestCatalog>::new_lazy(manifest()));
	app.update();
	assert!(app.world().resource::<Observations>().required.is_empty());
	assert_eq!(app.world().resource::<Observations>().optional, [None]);
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.desired()
			.is_empty()
	);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	app.update();
	assert_eq!(app.world().resource::<Observations>().required, ["ja"]);
	assert_eq!(
		app.world().resource::<Observations>().optional[1].as_deref(),
		Some("ja")
	);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.unload::<TestCatalog>();
	app.update();
	assert_eq!(app.world().resource::<Observations>().required.len(), 1);
	assert_eq!(app.world().resource::<Observations>().optional[2], None);
	assert!(!app.world().contains_resource::<TestCatalog>());

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
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
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new(manifest()))
		.add_localized_systems(Update, required);
	app.update();
	assert_eq!(app.world().resource::<Observations>().required, ["ja"]);
}

#[test]
fn same_schedule_resource_removal_delays_a_required_consumer() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new(manifest()));
	app.update();
	app.add_systems(
		Update,
		(
			(|world: &mut World| {
				world.remove_resource::<TestCatalog>();
			}),
			localized(required),
		)
			.chain(),
	);
	app.update();
	assert!(app.world().resource::<Observations>().required.is_empty());
	// PostUpdate restores the controller's still-owned snapshot.
	assert!(app.world().contains_resource::<TestCatalog>());
}

#[test]
fn inferred_condition_does_not_conflict_with_unrelated_writes() {
	use crate::bevy::ecs::system::System;
	use crate::compatibility::readiness::Readiness;

	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new(manifest()));
	let mut gate = IntoSystem::into_system(
		|ready: Readiness<(Res<TestCatalog>, ResMut<Observations>)>| ready.ready(),
	);
	let mut unrelated = IntoSystem::into_system(|_: ResMut<Observations>, _: Query<&mut Text>| {});

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
fn deferred_setup_waits_runs_once_and_applies_commands() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new_lazy(manifest()))
		.add_localized_startup_systems(setup);
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Observations>().setups, 0);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	app.update();
	assert_eq!(app.world().resource::<Observations>().setups, 1);
	assert_eq!(
		app.world_mut()
			.query::<&Created>()
			.iter(app.world())
			.count(),
		1
	);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.set_locale("es");
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Observations>().setups, 1);
}

#[test]
fn startup_tuple_members_wait_independently() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new_lazy(manifest()))
		.add_localized_startup_systems((setup, optional));
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Observations>().optional, [None]);
	assert_eq!(app.world().resource::<Observations>().setups, 0);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	app.update();
	assert_eq!(app.world().resource::<Observations>().setups, 1);
	assert_eq!(app.world().resource::<Observations>().optional, [None]);
}

#[test]
fn original_function_ordering_and_bevy_configuration_are_preserved() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new(manifest()))
		.add_systems(Update, localized(required).run_if(|| true))
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
fn a_locale_change_within_update_cannot_expose_the_previous_snapshot() {
	let mut app = app();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new(manifest()));
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	app.add_systems(
		Update,
		(
			|mut localization: ResMut<Localization<TestCatalog>>| localization.set_locale("es"),
			localized(required),
		)
			.chain(),
	);
	app.update();
	assert!(app.world().resource::<Observations>().required.is_empty());
	app.update();
	assert_eq!(app.world().resource::<Observations>().required, ["es"]);
}

#[test]
#[should_panic(expected = "startup schedules cannot retry")]
fn recurring_helper_rejects_startup_instead_of_losing_the_system() {
	app().add_localized_systems(Startup, required);
}

#[test]
fn ordinary_missing_resources_are_not_silenced() {
	let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
		let mut app = app();
		app.add_plugins(LocalizationPlugin::<TestCatalog>::new(manifest()))
			.add_localized_systems(Update, |_: Res<TestCatalog>, _: Res<Unrelated>| {});
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
			app.add_plugins(LocalizationPlugin::<TestCatalog>::new(manifest()));

			if once {
				app.add_localized_startup_systems(fail);
			} else {
				app.add_localized_systems(Update, fail);
			}

			app.update();
		}));
		assert!(result.is_err());
	}
}

#[test]
#[cfg(feature = "bevy-0-19")]
fn deferred_setup_returning_an_error_is_not_repeated_by_a_nonpanicking_handler() {
	use crate::bevy::ecs::error::{FallbackErrorHandler, ignore};

	fn fail_once(_: Res<TestCatalog>, mut observations: ResMut<Observations>) -> Result {
		observations.setups += 1;
		Err("one attempted initialization".into())
	}

	let mut app = app();
	app.insert_resource(FallbackErrorHandler(ignore))
		.add_plugins(LocalizationPlugin::<TestCatalog>::new_lazy(manifest()))
		.add_localized_startup_systems(fail_once);
	app.update();
	assert_eq!(app.world().resource::<Observations>().setups, 0);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
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
		LocalizationPlugin::<TestCatalog>::new(manifest()),
		LocalizationPlugin::<OtherCatalog>::new_lazy(other),
	))
	.add_localized_systems(
		Update,
		(
			|_: Res<TestCatalog>, _: Res<OtherCatalog>, mut observations: ResMut<Observations>| {
				observations.required.push("both".into());
			},
			|_: Res<TestCatalog>,
			 other: Option<Res<OtherCatalog>>,
			 mut observations: ResMut<Observations>| {
				observations.optional.push(other.map(|_| "other".into()));
			},
		),
	);
	app.update();
	assert!(app.world().resource::<Observations>().required.is_empty());
	assert_eq!(app.world().resource::<Observations>().optional, [None]);
	app.world_mut()
		.resource_mut::<Localization<OtherCatalog, Lazy>>()
		.load::<OtherCatalog>();
	app.update();
	assert_eq!(app.world().resource::<Observations>().required, ["both"]);
}
