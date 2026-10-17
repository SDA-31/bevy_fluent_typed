//! Native snapshots count physical demand rather than the complete Manual schema.
use super::{Hud, Other, Panel, Presentation, Root, configured, pump};
use crate::bevy::prelude::*;
use crate::{
	FluentScope, Localization, LocalizationPlugin, LocalizationProgress,
	LocalizationProgressPlugin, Manual, ModuleStatus, PreparationStatus,
};

type Progress = LocalizationProgress<Root>;

fn tracked() -> (App, super::Source) {
	let (mut app, source) = configured();
	app.add_plugins(LocalizationProgressPlugin::<Root>::new());
	app.finish();
	app.cleanup();
	(app, source)
}

#[test]
fn leases_and_manual_pins_count_their_unique_union_without_loading_siblings() {
	let (mut app, source) = tracked();
	assert_eq!(app.world().resource::<Progress>().active().total, 0);
	assert_eq!(
		app.world()
			.resource::<Localization<Root, Manual>>()
			.progress::<Root>()
			.unloaded,
		3
	);
	app.update();
	assert_eq!(source.count(), 0);
	let group = app
		.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.hold::<Presentation>();
	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.load::<Hud>();
	pump(&mut app, |_| source.count() == 2);
	assert_eq!(app.world().resource::<Progress>().active().total, 2);
	assert_eq!(app.world().resource::<Progress>().active().loading, 2);
	source.release::<Hud>("en", 0);
	pump(&mut app, |world| {
		world.resource::<Progress>().active().ready == 1
	});
	let active = app.world().resource::<Progress>().active();
	assert_eq!(active.available, 1);
	assert_eq!(active.loading, 1);
	assert_eq!(active.unloaded, 0);
	assert_eq!(active.failed, 0);
	assert!(!app.world().contains_resource::<Other>());

	drop(group);
	app.update();
	let active = app.world().resource::<Progress>().active();
	assert_eq!(active.total, 1);
	assert_eq!(active.ready, 1);
	assert_eq!(active.available, 1);
	assert!(!app.world().contains_resource::<Panel>());
	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.unload::<Hud>();
	app.update();
	assert_eq!(app.world().resource::<Progress>().active().total, 0);
	assert_eq!(app.world().resource::<Progress>().active().available, 0);

	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.prepare_locale("es");
	app.update();
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.preparation().unwrap().total, 0);
	assert_eq!(progress.preparation_status(), &PreparationStatus::Ready);
	assert_eq!(source.count(), 2);
}

#[test]
fn late_update_requests_publish_before_postupdate_observers() {
	let (mut app, _) = tracked();
	let mut once = true;
	app.add_systems(
		Update,
		move |mut state: ResMut<Localization<Root, Manual>>| {
			if !once {
				return;
			}

			once = false;
			state.load::<Hud>();
		},
	);
	app.add_systems(
		PostUpdate,
		(|progress: Res<Progress>| {
			assert_eq!(progress.active().total, 1);
			assert_eq!(progress.active().loading, 1);
		})
		.after(crate::LocalizationSystems::Progress),
	);
	app.update();
}

#[test]
fn changed_failure_details_do_not_tick_unchanged_loading_counts() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<Root, Manual>::from_loader(|_, _| {
			std::future::ready(Err::<Vec<u8>, String>("offline".into()))
		}),
	))
	.add_plugins(LocalizationProgressPlugin::<Root>::new());
	app.finish();
	app.cleanup();
	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.load::<Presentation>();
	pump(&mut app, |world| {
		world.resource::<Progress>().active().failed == 2
	});
	let before = app.world().resource::<Progress>().active().clone();
	let tick = app
		.world()
		.get_resource_ref::<Progress>()
		.unwrap()
		.last_changed();
	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.store
		.set_status(
			Hud::module_paths()[0],
			ModuleStatus::Failed("updated diagnostic detail".into()),
		);
	app.update();
	assert_eq!(app.world().resource::<Progress>().active(), &before);
	assert_eq!(
		app.world()
			.get_resource_ref::<Progress>()
			.unwrap()
			.last_changed(),
		tick
	);
	#[cfg(feature = "diagnostics")]
	assert_eq!(
		app.world()
			.resource::<Localization<Root, Manual>>()
			.diagnostics::<Hud>()[0]
			.status,
		ModuleStatus::Failed("updated diagnostic detail".into())
	);
}
