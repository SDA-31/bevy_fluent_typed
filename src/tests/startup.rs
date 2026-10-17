use super::{TestCatalog, bytes::pump};
use crate::bevy::{
	ecs::{self as bevy_ecs, schedule::SingleThreadedExecutor},
	prelude::*,
};
use crate::{LocalizationPlugin, LocalizationStartupPlugin, Localized, StartupStrategy};
use std::sync::{Arc, Mutex};

#[derive(Resource, Default)]
struct Steps(Vec<&'static str>);

fn services(mut steps: ResMut<Steps>) {
	steps.0.push("services");
}

fn hud(_: Res<TestCatalog>, mut steps: ResMut<Steps>, mut commands: Commands) {
	steps.0.push("hud");
	commands.insert_resource(Initialized);
}

#[derive(Resource)]
struct Initialized;

fn finish(_: Res<Initialized>, mut steps: ResMut<Steps>) {
	steps.0.push("finish");
}

fn post(mut steps: ResMut<Steps>) {
	steps.0.push("post");
}

fn app<M: crate::LoadingMode>(plugin: LocalizationPlugin<TestCatalog, M>) -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin))
		.init_resource::<Steps>();
	app
}

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

fn delayed_plugin(allow: Arc<Gate>) -> LocalizationPlugin<TestCatalog> {
	LocalizationPlugin::<TestCatalog>::from_loader(move |locale, _| {
		let allow = allow.clone();
		async move {
			std::future::poll_fn(|context| {
				let mut gate = allow.0.lock().unwrap();
				if !gate.0 {
					gate.1 = Some(context.waker().clone());
					std::task::Poll::Pending
				} else {
					std::task::Poll::Ready(Ok::<_, String>(locale.as_bytes().to_vec()))
				}
			})
			.await
		}
	})
}

#[test]
fn deferred_startup_runs_each_step_once_preserves_dependencies_and_poststartup() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.add_systems(Startup, (services, hud, finish).chain())
		.add_systems(PostStartup, post);
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["services"]);
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	app.update();
	assert_eq!(
		app.world().resource::<Steps>().0,
		["services", "hud", "finish", "post"]
	);
}

fn complete_setup(mut steps: ResMut<Steps>) {
	steps.0.push("finish");
}

#[test]
fn explicit_apply_deferred_completes_startup_without_catalog_reads() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_loader(|_, _| {
		std::future::pending::<Result<Vec<u8>, String>>()
	}));
	app.add_systems(Startup, (services, ApplyDeferred, complete_setup).chain())
		.add_systems(PostStartup, post);

	for _ in 0..3 {
		app.update();
	}

	assert_eq!(
		app.world().resource::<Steps>().0,
		["services", "finish", "post"]
	);
}

#[test]
fn explicit_apply_deferred_applies_commands_after_catalog_and_native_condition() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.add_systems(
		Startup,
		(hud, ApplyDeferred.run_if(|| true), finish).chain_ignore_deferred(),
	)
	.add_systems(PostStartup, post)
	.edit_schedule(Startup, |schedule| {
		schedule
			.set_executor(SingleThreadedExecutor::new())
			.set_apply_final_deferred(false);
	});
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	assert!(!app.world().contains_resource::<Initialized>());

	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["hud", "finish", "post"]);
}

#[test]
fn explicit_apply_deferred_waits_for_a_catalog_in_its_condition() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.add_systems(
		Startup,
		(
			services,
			ApplyDeferred.run_if(|_: Res<TestCatalog>| true),
			complete_setup,
		)
			.chain_ignore_deferred(),
	)
	.add_systems(PostStartup, post);
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["services"]);

	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	app.update();
	assert_eq!(
		app.world().resource::<Steps>().0,
		["services", "finish", "post"]
	);
}

#[test]
fn skipped_apply_deferred_does_not_hold_startup_open() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_loader(|_, _| {
		std::future::pending::<Result<Vec<u8>, String>>()
	}));
	app.add_systems(
		Startup,
		(services, ApplyDeferred.run_if(|| false), complete_setup).chain(),
	)
	.add_systems(PostStartup, post);
	app.update();
	app.update();
	assert_eq!(
		app.world().resource::<Steps>().0,
		["services", "finish", "post"]
	);
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AppStartup;

#[test]
fn barrier_holds_selected_independent_steps_but_runs_outside_services() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.add_plugins(LocalizationStartupPlugin::barrier(AppStartup))
		.add_systems(Startup, services)
		.add_systems(Startup, (hud, finish).chain().in_set(AppStartup))
		.add_systems(PostStartup, post);
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["services"]);
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(
		app.world().resource::<Steps>().0,
		["services", "hud", "finish", "post"]
	);
}

#[test]
fn native_startup_can_create_bindings_without_waiting_for_catalog() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap());
	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Native))
		.add_systems(Startup, |mut commands: Commands| {
			commands.spawn(Localized::<Text, TestCatalog>::new(|catalog| {
				catalog.0.clone()
			}));
		})
		.add_systems(PostStartup, post);
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["post"]);
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
}

#[test]
fn a_false_native_startup_condition_settles_without_hanging_poststartup() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap());
	app.add_systems(Startup, services.run_if(|| false))
		.add_systems(PostStartup, post);
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["post"]);
}

#[test]
fn false_condition_does_not_wait_for_a_catalog_that_never_completes() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_loader(|_, _| {
		std::future::pending::<Result<Vec<u8>, String>>()
	}));
	app.add_systems(Startup, hud.run_if(|| false))
		.add_systems(PostStartup, post);
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["post"]);
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn required_catalog_in_a_system_condition_waits_before_evaluating() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.add_systems(
		Startup,
		services.run_if(|scope: Res<TestCatalog>| scope.0 == "ja"),
	)
	.add_systems(PostStartup, post);
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(app.world().resource::<Steps>().0, ["services", "post"]);
}

#[test]
fn required_catalog_in_a_set_condition_waits_before_evaluating() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.configure_sets(
		Startup,
		AppStartup.run_if(|scope: Res<TestCatalog>| scope.0 == "ja"),
	)
	.add_systems(Startup, services.in_set(AppStartup))
	.add_systems(PostStartup, post);
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(app.world().resource::<Steps>().0, ["services", "post"]);
}

#[test]
fn dependent_native_condition_waits_for_commands_from_a_catalog_consumer() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.add_systems(
		Startup,
		(hud, finish.run_if(resource_exists::<Initialized>)).chain(),
	)
	.add_systems(PostStartup, post);
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(app.world().resource::<Steps>().0, ["hud", "finish", "post"]);
}

#[test]
fn dependent_required_resource_condition_is_not_validated_before_its_predecessor() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.add_systems(
		Startup,
		(hud, finish.run_if(|_: Res<Initialized>| true)).chain(),
	)
	.add_systems(PostStartup, post);
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(app.world().resource::<Steps>().0, ["hud", "finish", "post"]);
}

#[test]
fn shared_native_condition_waits_for_external_predecessors() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.configure_sets(Startup, AppStartup.run_if(resource_exists::<Initialized>))
		.add_systems(Startup, hud)
		.add_systems(Startup, finish.in_set(AppStartup).after(hud))
		.add_systems(PostStartup, post);
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(app.world().resource::<Steps>().0, ["hud", "finish", "post"]);
}

#[test]
fn shared_condition_does_not_wait_for_external_successors_of_its_first_member() {
	use std::sync::atomic::{AtomicUsize, Ordering};
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	let evaluations = Arc::new(AtomicUsize::new(0));
	let seen = evaluations.clone();
	app.configure_sets(
		Startup,
		AppStartup.run_if(move || {
			seen.fetch_add(1, Ordering::Relaxed);
			true
		}),
	)
	.add_systems(Startup, services.in_set(AppStartup))
	.add_systems(Startup, hud.after(services))
	.add_systems(Startup, finish.in_set(AppStartup).after(hud))
	.add_systems(PostStartup, post);
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["services"]);
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(
		app.world().resource::<Steps>().0,
		["services", "hud", "finish", "post"]
	);
	assert_eq!(evaluations.load(Ordering::Relaxed), 1);
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct OriginalStartup;

#[test]
fn late_startup_set_member_owns_condition_demand_after_original_is_removed() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.configure_sets(Startup, AppStartup.run_if(|_: Res<TestCatalog>| true))
		.add_systems(Startup, services.in_set(AppStartup).in_set(OriginalStartup))
		.add_systems(PostStartup, post);
	app.update();
	app.add_systems(Startup, selected_after_external.in_set(AppStartup));
	app.update();
	app.world_mut().schedule_scope(Startup, |world, schedule| {
		schedule
			.remove_systems_in_set(
				OriginalStartup,
				world,
				crate::bevy::ecs::schedule::ScheduleCleanupPolicy::RemoveSystemsOnly,
			)
			.unwrap();
	});
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(app.world().resource::<Steps>().0, ["selected", "post"]);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn manual_mode_waits_for_explicit_load_without_creating_automatic_demand() {
	let plugin = LocalizationPlugin::<TestCatalog, crate::Manual>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap();
	let mut app = app(plugin);
	app.add_systems(Startup, hud).add_systems(PostStartup, post);
	app.update();
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	assert!(
		app.world()
			.resource::<crate::Localization<TestCatalog, crate::Manual>>()
			.desired()
			.is_empty()
	);
	app.world_mut()
		.resource_mut::<crate::Localization<TestCatalog, crate::Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
}

#[test]
fn full_mode_waits_for_preloaded_catalogs_in_ordinary_startup() {
	let plugin = LocalizationPlugin::<TestCatalog, crate::Full>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap();
	let mut app = app(plugin);
	app.add_systems(Startup, hud).add_systems(PostStartup, post);
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(app.world().resource::<Steps>().0, ["hud", "post"]);
}

#[test]
fn startup_policy_is_shared_by_multiple_roots_in_both_registration_orders() {
	use super::native_resources::FtlCatalog;

	fn optional_catalog(_: Option<Res<TestCatalog>>, mut steps: ResMut<Steps>) {
		steps.0.push("optional");
	}

	for explicit_first in [false, true] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_resource::<Steps>();
		let first = delayed_plugin(Arc::new(Gate::default()));
		let second = LocalizationPlugin::<FtlCatalog>::from_bytes([(
			"en",
			"hud.ftl",
			b"greeting = Hello\n".as_slice(),
		)])
		.unwrap();
		let startup = LocalizationStartupPlugin::new(StartupStrategy::Native);

		if explicit_first {
			app.add_plugins(startup).add_plugins((first, second));
		} else {
			app.add_plugins((first, second)).add_plugins(startup);
		}

		app.add_systems(Startup, (services, optional_catalog).chain())
			.add_systems(PostStartup, post);
		app.update();
		app.update();
		assert_eq!(
			app.world().resource::<Steps>().0,
			["services", "optional", "post"]
		);
		assert!(!app.world().contains_resource::<TestCatalog>());
	}
}

#[test]
#[should_panic(expected = "conflicting startup strategies")]
fn contradictory_explicit_startup_plugins_are_rejected() {
	let mut app = App::new();
	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Native));
	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Deferred));
}

#[test]
fn public_startup_plugin_overrides_fallback_in_both_registration_orders() {
	fn optional_catalog(_: Option<Res<TestCatalog>>, mut steps: ResMut<Steps>) {
		steps.0.push("optional");
	}

	for explicit_first in [false, true] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_resource::<Steps>();
		let root = delayed_plugin(Arc::new(Gate::default()));
		let startup = LocalizationStartupPlugin::new(StartupStrategy::Native);

		if explicit_first {
			app.add_plugins(startup).add_plugins(root);
			assert_eq!(
				app.get_added_plugins::<LocalizationStartupPlugin>().len(),
				1
			);
		} else {
			app.add_plugins(root).add_plugins(startup);
			assert_eq!(
				app.get_added_plugins::<LocalizationStartupPlugin>().len(),
				2
			);
		}

		app.add_systems(Startup, optional_catalog)
			.add_systems(PostStartup, post);
		app.update();
		app.update();
		assert_eq!(app.world().resource::<Steps>().0, ["optional", "post"]);
		assert!(!app.world().contains_resource::<TestCatalog>());
	}
}

#[test]
fn multiple_roots_and_matching_startup_plugins_install_one_main_driver() {
	use super::native_resources::FtlCatalog;
	use crate::bevy::app::Main;

	let mut app = app(delayed_plugin(Arc::new(Gate::default())));
	assert_eq!(
		app.get_added_plugins::<LocalizationStartupPlugin>().len(),
		1
	);
	let original = app
		.world()
		.resource::<Schedules>()
		.get(Main)
		.unwrap()
		.graph()
		.systems
		.len();

	app.add_plugins(
		LocalizationPlugin::<FtlCatalog>::from_bytes([(
			"en",
			"hud.ftl",
			b"greeting = Hello\n".as_slice(),
		)])
		.unwrap(),
	);
	assert_eq!(
		app.get_added_plugins::<LocalizationStartupPlugin>().len(),
		1
	);

	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Deferred));
	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Deferred));
	assert_eq!(
		app.world()
			.resource::<Schedules>()
			.get(Main)
			.unwrap()
			.graph()
			.systems
			.len(),
		original
	);
	app.add_systems(Startup, services)
		.add_systems(PostStartup, post);
	app.update();
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["services", "post"]);
}

#[test]
fn public_startup_barrier_waits_for_the_selected_set_in_both_registration_orders() {
	for explicit_first in [false, true] {
		let allow = Arc::new(Gate::default());
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).init_resource::<Steps>();
		let root = delayed_plugin(allow.clone());
		let startup = LocalizationStartupPlugin::barrier(AppStartup);

		if explicit_first {
			app.add_plugins(startup).add_plugins(root);
		} else {
			app.add_plugins(root).add_plugins(startup);
		}

		app.add_systems(Startup, services)
			.add_systems(
				Startup,
				(hud, |mut steps: ResMut<Steps>| steps.0.push("selected")).in_set(AppStartup),
			)
			.add_systems(PostStartup, post);
		app.update();
		app.update();
		assert_eq!(app.world().resource::<Steps>().0, ["services"]);

		allow.open();
		pump(&mut app, |world| {
			world.resource::<Steps>().0.contains(&"post")
		});
		app.update();
		for step in ["services", "hud", "selected", "post"] {
			assert_eq!(
				app.world()
					.resource::<Steps>()
					.0
					.iter()
					.filter(|observed| **observed == step)
					.count(),
				1
			);
		}
	}
}

#[test]
fn public_startup_plugin_can_run_without_localization_roots() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationStartupPlugin::new(StartupStrategy::Deferred),
	))
	.init_resource::<Steps>()
	.add_systems(Startup, (services, complete_setup).chain())
	.add_systems(PostStartup, post);
	app.update();
	app.update();
	assert_eq!(
		app.world().resource::<Steps>().0,
		["services", "finish", "post"]
	);
}

#[test]
#[should_panic(expected = "conflicting startup strategies")]
fn a_startup_barrier_rejects_a_conflicting_explicit_strategy() {
	let mut app = App::new();
	app.add_plugins(LocalizationStartupPlugin::barrier(AppStartup));
	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Native));
}

#[test]
#[should_panic(expected = "conflicting startup strategies")]
fn an_explicit_strategy_rejects_a_conflicting_startup_barrier() {
	let mut app = App::new();
	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Deferred));
	app.add_plugins(LocalizationStartupPlugin::barrier(AppStartup));
}

#[test]
#[should_panic(expected = "startup strategy cannot change after the application has started")]
fn public_startup_policy_cannot_be_added_after_startup_begins() {
	let mut app = app(delayed_plugin(Arc::new(Gate::default())));
	app.update();
	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Native));
}

#[derive(crate::bevy::ecs::schedule::ScheduleLabel, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SharedPhase;

#[derive(Resource, Default)]
struct SharedRuns(usize);

#[test]
fn a_schedule_shared_by_startup_and_main_remains_recurring() {
	use crate::bevy::ecs::schedule::ScheduleLabel;
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap());
	app.init_resource::<SharedRuns>()
		.add_systems(SharedPhase, |mut count: ResMut<SharedRuns>| count.0 += 1)
		.add_systems(PostStartup, post);
	{
		let mut order = app
			.world_mut()
			.resource_mut::<crate::bevy::app::MainScheduleOrder>();
		order.startup_labels.insert(1, SharedPhase.intern());
		order.labels.push(SharedPhase.intern());
	}
	app.update();
	assert_eq!(app.world().resource::<SharedRuns>().0, 2);
	assert_eq!(app.world().resource::<Steps>().0, ["post"]);
	app.update();
	assert_eq!(app.world().resource::<SharedRuns>().0, 3);
	app.update();
	assert_eq!(app.world().resource::<SharedRuns>().0, 4);
}

#[test]
fn startup_registered_after_completion_does_not_acquire_demand_or_run() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap());
	app.update();
	app.add_systems(Startup, hud);
	app.update();
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	assert!(
		app.world()
			.resource::<crate::Localization<TestCatalog>>()
			.desired()
			.is_empty()
	);
}

#[derive(Resource)]
struct StartupInput;

struct InitializedLocal;

impl FromWorld for InitializedLocal {
	fn from_world(world: &mut World) -> Self {
		assert!(world.contains_resource::<StartupInput>());
		Self
	}
}

#[test]
fn later_phases_initialize_locals_after_previous_startup_commands_are_applied() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap());
	app.add_systems(PreStartup, |mut commands: Commands| {
		commands.insert_resource(StartupInput)
	});
	app.add_systems(Startup, |_: Local<InitializedLocal>| {});
	app.add_systems(Update, |_: Local<InitializedLocal>| {});
	app.update();
}

#[test]
fn native_startup_also_initializes_update_locals_after_startup_commands() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap());
	app.add_plugins(LocalizationStartupPlugin::new(StartupStrategy::Native))
		.add_systems(Startup, |mut commands: Commands| {
			commands.insert_resource(StartupInput)
		});
	app.add_systems(Update, |_: Local<InitializedLocal>| {});
	app.update();
}

#[test]
fn barrier_holds_an_independent_selected_system_until_catalogs_are_ready() {
	let allow = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(allow.clone()));
	app.add_plugins(LocalizationStartupPlugin::barrier(AppStartup))
		.add_systems(Startup, services)
		.add_systems(
			Startup,
			(hud, |mut steps: ResMut<Steps>| steps.0.push("selected")).in_set(AppStartup),
		)
		.add_systems(PostStartup, post);
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["services"]);
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(
		app.world()
			.resource::<Steps>()
			.0
			.iter()
			.filter(|step| **step == "selected")
			.count(),
		1
	);
}

#[test]
fn removing_a_waiting_startup_system_releases_demand_and_allows_poststartup() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_loader(|_, _| {
		std::future::pending::<Result<Vec<u8>, String>>()
	}));
	app.add_systems(Startup, hud.in_set(AppStartup))
		.add_systems(PostStartup, post);
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	app.world_mut().schedule_scope(Startup, |world, schedule| {
		assert_eq!(
			schedule
				.remove_systems_in_set(
					AppStartup,
					world,
					crate::bevy::ecs::schedule::ScheduleCleanupPolicy::RemoveSystemsOnly
				)
				.unwrap(),
			1
		);
	});
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["post"]);
	assert!(
		app.world()
			.resource::<crate::Localization<TestCatalog>>()
			.desired()
			.is_empty()
	);
}

#[test]
fn startup_commands_hand_off_demand_to_inserted_bindings_without_refetch() {
	use std::sync::atomic::{AtomicUsize, Ordering};
	let reads = Arc::new(AtomicUsize::new(0));
	let observed = reads.clone();
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_loader(
		move |locale, _| {
			observed.fetch_add(1, Ordering::Relaxed);
			std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
		},
	));
	app.add_systems(Startup, |_: Res<TestCatalog>, mut commands: Commands| {
		commands.spawn(Localized::<Text, TestCatalog>::new(|scope| scope.0.clone()));
	})
	.add_systems(PostStartup, post);
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	assert_eq!(reads.load(Ordering::Relaxed), 1);
}

#[test]
fn a_failed_startup_load_stays_pending_until_explicit_retry_then_runs_once() {
	use std::sync::atomic::{AtomicUsize, Ordering};
	let attempts = Arc::new(AtomicUsize::new(0));
	let observed = attempts.clone();
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_loader(
		move |locale, _| {
			let attempt = observed.fetch_add(1, Ordering::Relaxed);
			std::future::ready(if attempt == 0 {
				Err("failed read".to_owned())
			} else {
				Ok(locale.as_bytes().to_vec())
			})
		},
	));
	app.add_systems(Startup, hud).add_systems(PostStartup, post);
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<crate::Localization<TestCatalog>>()
				.status::<TestCatalog>(),
			crate::ModuleStatus::Failed(_)
		)
	});
	assert!(app.world().resource::<Steps>().0.is_empty());
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["hud", "post"]);
	assert_eq!(attempts.load(Ordering::Relaxed), 2);
}

fn delayed_external(_: Res<super::native_resources::FtlCatalog>, mut steps: ResMut<Steps>) {
	steps.0.push("external");
}

fn selected_after_external(mut steps: ResMut<Steps>) {
	steps.0.push("selected");
}

#[test]
fn barrier_retains_its_catalog_while_a_selected_dependent_waits_on_an_outside_step() {
	use super::native_resources::FtlCatalog;
	let allow = Arc::new(Gate::default());
	let gate = allow.clone();
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap());
	app.add_plugins(LocalizationStartupPlugin::barrier(AppStartup));
	app.add_plugins(LocalizationPlugin::<FtlCatalog>::from_loader(
		move |_, _| {
			let gate = gate.clone();
			async move {
				std::future::poll_fn(|context| {
					let mut state = gate.0.lock().unwrap();
					if state.0 {
						std::task::Poll::Ready(Ok::<_, String>(b"greeting = Hello\n".to_vec()))
					} else {
						state.1 = Some(context.waker().clone());
						std::task::Poll::Pending
					}
				})
				.await
			}
		},
	));
	app.add_systems(Startup, hud.in_set(AppStartup))
		.add_systems(Startup, delayed_external)
		.add_systems(
			Startup,
			selected_after_external
				.in_set(AppStartup)
				.after(delayed_external),
		)
		.add_systems(PostStartup, post);
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"hud")
	});
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	assert!(!app.world().resource::<Steps>().0.contains(&"selected"));
	allow.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(
		app.world().resource::<Steps>().0,
		["hud", "external", "selected", "post"]
	);
}

#[test]
fn update_locals_initialize_after_preupdate_commands() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap());
	app.add_systems(PreUpdate, |mut commands: Commands| {
		commands.insert_resource(StartupInput)
	});
	app.add_systems(Update, |_: Local<InitializedLocal>| {});
	app.update();
}

#[test]
fn removing_a_waiting_set_member_releases_condition_owned_catalog_demand() {
	let mut app = app(LocalizationPlugin::<TestCatalog>::from_loader(|_, _| {
		std::future::pending::<Result<Vec<u8>, String>>()
	}));
	app.configure_sets(Startup, AppStartup.run_if(|_: Res<TestCatalog>| true))
		.add_systems(Startup, services.in_set(AppStartup))
		.add_systems(PostStartup, post);
	app.update();
	app.world_mut().schedule_scope(Startup, |world, schedule| {
		schedule
			.remove_systems_in_set(
				AppStartup,
				world,
				crate::bevy::ecs::schedule::ScheduleCleanupPolicy::RemoveSystemsOnly,
			)
			.unwrap();
	});
	app.update();
	assert_eq!(app.world().resource::<Steps>().0, ["post"]);
	assert!(
		app.world()
			.resource::<crate::Localization<TestCatalog>>()
			.desired()
			.is_empty()
	);
}

fn delayed_ftl_plugin(gate: Arc<Gate>) -> LocalizationPlugin<super::native_resources::FtlCatalog> {
	LocalizationPlugin::<super::native_resources::FtlCatalog>::from_loader(move |_, _| {
		let gate = gate.clone();
		async move {
			std::future::poll_fn(|context| {
				let mut state = gate.0.lock().unwrap();
				if state.0 {
					std::task::Poll::Ready(Ok::<_, String>(b"greeting = Hello\n".to_vec()))
				} else {
					state.1 = Some(context.waker().clone());
					std::task::Poll::Pending
				}
			})
			.await
		}
	})
}

#[test]
fn a_new_barrier_member_updates_the_wait_of_existing_unfinished_members() {
	let first = Arc::new(Gate::default());
	let second = Arc::new(Gate::default());
	let mut app = app(delayed_plugin(first.clone()));
	app.add_plugins(LocalizationStartupPlugin::barrier(AppStartup))
		.add_plugins(delayed_ftl_plugin(second.clone()))
		.add_systems(Startup, hud.in_set(AppStartup))
		.add_systems(PostStartup, post);
	app.update();
	app.add_systems(Startup, delayed_external.in_set(AppStartup));
	first.open();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.update();
	assert!(app.world().resource::<Steps>().0.is_empty());
	second.open();
	pump(&mut app, |world| {
		world.resource::<Steps>().0.contains(&"post")
	});
	assert_eq!(
		app.world()
			.resource::<Steps>()
			.0
			.iter()
			.filter(|step| **step == "hud")
			.count(),
		1
	);
	assert_eq!(
		app.world()
			.resource::<Steps>()
			.0
			.iter()
			.filter(|step| **step == "external")
			.count(),
		1
	);
}
