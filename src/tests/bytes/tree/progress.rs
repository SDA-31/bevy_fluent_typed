//! Native snapshots count physical demand rather than the complete Lazy schema.
use super::{Hud, Other, Panel, Presentation, Root, controlled, pump};
use crate::bevy::prelude::*;
use crate::{Lazy, Localization, LocalizationProgress, PreparationStatus};

type Progress = LocalizationProgress<Root>;

#[test]
fn leases_and_manual_pins_count_their_unique_union_without_loading_siblings() {
	let (mut app, source) = controlled();
	assert_eq!(app.world().resource::<Progress>().active().total, 0);
	assert_eq!(
		app.world()
			.resource::<Localization<Root, Lazy>>()
			.progress::<Root>()
			.unloaded,
		3
	);
	app.update();
	assert_eq!(source.count(), 0);
	let group = app
		.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.hold::<Presentation>();
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
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
	assert_eq!(active.modules.len(), 2);
	assert!(!app.world().contains_resource::<Other>());

	drop(group);
	app.update();
	let active = app.world().resource::<Progress>().active();
	assert_eq!(active.total, 1);
	assert_eq!(active.ready, 1);
	assert_eq!(active.available, 1);
	assert!(!app.world().contains_resource::<Panel>());
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.unload::<Hud>();
	app.update();
	assert_eq!(app.world().resource::<Progress>().active().total, 0);
	assert_eq!(app.world().resource::<Progress>().active().available, 0);

	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.prepare_locale("es");
	app.update();
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.preparation().unwrap().total, 0);
	assert_eq!(progress.preparation_status(), &PreparationStatus::Ready);
	assert_eq!(source.count(), 2);
}

#[test]
fn late_update_requests_publish_before_postupdate_observers() {
	let (mut app, _) = controlled();
	let mut once = true;
	app.add_systems(
		Update,
		move |mut state: ResMut<Localization<Root, Lazy>>| {
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
		.after(crate::LocalizationSystems::Refresh),
	);
	app.update();
}
