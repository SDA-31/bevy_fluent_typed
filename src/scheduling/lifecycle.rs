use super::{
	configuration::StartupConfiguration,
	startup::{self, KnownSchedules, Tracking},
};
use crate::bevy::{
	app::{Main, MainScheduleOrder},
	ecs::{
		self as bevy_ecs,
		schedule::{InternedScheduleLabel, ScheduleLabel},
	},
	prelude::{
		App, IntoScheduleConfigs, Mut, PostStartup, PreStartup, Resource, Schedules, Startup, World,
	},
};
use std::collections::HashMap;

/// Single App-wide startup driver, independent of individual localization providers.
/// Owns the pending startup phase and discovery records for installed schedules.
#[derive(Resource, Default)]
struct Coordinator {
	explicit: Option<StartupConfiguration>,
	started: bool,
	startup: Vec<InternedScheduleLabel>,
	phase: usize,
	schedules: HashMap<InternedScheduleLabel, Tracking>,
	known: KnownSchedules,
}

pub(crate) fn register(app: &mut App, configuration: Option<StartupConfiguration>) {
	let first = !app.world().contains_resource::<Coordinator>();
	app.init_resource::<Coordinator>();
	let mut coordinator = app.world_mut().resource_mut::<Coordinator>();
	if let Some(configuration) = configuration {
		assert!(
			!coordinator.started,
			"startup strategy cannot change after the application has started"
		);
		assert!(
			coordinator
				.explicit
				.is_none_or(|previous| previous == configuration),
			"localization startup plugins specify conflicting startup strategies"
		);
		coordinator.explicit = Some(configuration);
	}
	if first {
		app.add_systems(Main, drive.before(Main::run_main));
	}
}

fn drive(world: &mut World) {
	world.resource_scope(|world, mut coordinator: Mut<Coordinator>| {
		let policy = coordinator.explicit.unwrap_or_default();
		if !coordinator.started {
			coordinator.started = true;
			coordinator.startup =
				std::mem::take(&mut world.resource_mut::<MainScheduleOrder>().startup_labels);
		}
		let labels: Vec<_> = world
			.resource::<Schedules>()
			.iter()
			.map(|(_, schedule)| schedule.label())
			.filter(|label| *label != Main.intern())
			.collect();
		coordinator
			.schedules
			.retain(|label, _| labels.contains(label));
		let recurring = world.resource::<MainScheduleOrder>().labels.clone();

		advance_startup(world, &mut coordinator, &labels, &recurring, policy);
		prepare_recurring(world, &mut coordinator, &labels, &recurring, policy);

		for tracking in coordinator.schedules.values() {
			startup::release_completed(tracking);
		}
	});
}

fn advance_startup(
	world: &mut World,
	coordinator: &mut Coordinator,
	labels: &[InternedScheduleLabel],
	recurring: &[InternedScheduleLabel],
	policy: StartupConfiguration,
) {
	while let Some(label) = coordinator.startup.get(coordinator.phase).copied() {
		if labels.contains(&label) {
			let Coordinator {
				schedules, known, ..
			} = &mut *coordinator;
			let tracking = schedules.entry(label).or_default();
			world.schedule_scope(label, |_, schedule| {
				startup::prepare(
					schedule,
					tracking,
					known,
					!recurring.contains(&label),
					policy,
				)
			});
		}
		let _ = world.try_run_schedule(label);
		let complete = recurring.contains(&label)
			|| coordinator
				.schedules
				.get(&label)
				.is_none_or(startup::finished);
		if !complete {
			return;
		}
		if !recurring.contains(&label)
			&& let Some(tracking) = coordinator.schedules.remove(&label)
		{
			startup::release_completed(&tracking);
			tracking.states.lock().unwrap().clear();
		}
		coordinator.phase += 1;
	}
}

fn prepare_recurring(
	world: &mut World,
	coordinator: &mut Coordinator,
	labels: &[InternedScheduleLabel],
	recurring: &[InternedScheduleLabel],
	policy: StartupConfiguration,
) {
	for &label in labels {
		if !recurring.contains(&label)
			&& (coordinator.startup.contains(&label)
				|| [PreStartup.intern(), Startup.intern(), PostStartup.intern()].contains(&label))
		{
			continue;
		}
		let Coordinator {
			schedules, known, ..
		} = &mut *coordinator;
		let tracking = schedules.entry(label).or_default();
		world.schedule_scope(label, |_, schedule| {
			startup::prepare(schedule, tracking, known, false, policy)
		});
	}
}
