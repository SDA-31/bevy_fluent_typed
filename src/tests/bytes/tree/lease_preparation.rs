//! Lease demand and its release queue survive explicit locale preparation.
use super::{Hud, Panel, Root, controlled, pump};
use crate::bevy::prelude::*;
use crate::{Localization, LocalizedText, Manual, PreparationStatus};

#[test]
fn new_lease_revokes_a_queued_commit_and_existing_owners_survive_publication() {
	let (mut app, source) = controlled();
	let (first, second) = {
		let mut state = app.world_mut().resource_mut::<Localization<Root, Manual>>();
		(state.hold::<Hud>(), state.hold::<Hud>())
	};
	let label = app
		.world_mut()
		.spawn(LocalizedText::<Hud>::new(|hud| hud.0.as_str().to_owned()))
		.id();
	pump(&mut app, |_| source.count() == 1);
	let active = source.release::<Hud>("en", 0);
	pump(&mut app, |world| {
		world
			.get::<Text>(label)
			.is_some_and(|text| text.0 == active)
	});

	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.prepare_locale("es");
	pump(&mut app, |_| source.count() == 2);
	let target = source.release::<Hud>("es", 0);
	pump(&mut app, |world| {
		world
			.resource::<Localization<Root, Manual>>()
			.preparation_status()
			== PreparationStatus::Ready
	});

	let panel = {
		let mut state = app.world_mut().resource_mut::<Localization<Root, Manual>>();
		state.commit_locale().unwrap();
		assert!(state.commit_requested);
		let panel = state.hold::<Panel>();
		assert!(!state.commit_requested);
		assert_eq!(state.preparation_status(), PreparationStatus::Preparing);
		panel
	};
	pump(&mut app, |_| source.count() == 4);
	assert_eq!(app.world().get::<Text>(label).unwrap().0, active);
	assert_eq!(
		app.world()
			.resource::<Localization<Root, Manual>>()
			.locale(),
		"en"
	);
	source.release::<Panel>("en", 0);
	source.release::<Panel>("es", 0);
	pump(&mut app, |world| {
		world
			.resource::<Localization<Root, Manual>>()
			.preparation_status()
			== PreparationStatus::Ready
	});

	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.commit_locale()
		.unwrap();
	drop(first);
	app.update();
	assert_eq!(
		app.world()
			.resource::<Localization<Root, Manual>>()
			.locale(),
		"es"
	);
	assert_eq!(app.world().get::<Text>(label).unwrap().0, target);
	assert!(app.world().contains_resource::<Hud>());
	assert!(app.world().contains_resource::<Panel>());

	drop(panel);
	app.update();
	assert!(!app.world().contains_resource::<Panel>());
	assert!(app.world().contains_resource::<Hud>());
	drop(second);
	app.update();
	assert!(!app.world().contains_resource::<Hud>());
	assert!(app.world().get::<Text>(label).unwrap().0.is_empty());
}

#[test]
fn dropping_a_target_owner_releases_its_pending_work_before_commit() {
	let (mut app, source) = controlled();
	let (hud, panel) = {
		let mut state = app.world_mut().resource_mut::<Localization<Root, Manual>>();
		(state.hold::<Hud>(), state.hold::<Panel>())
	};
	pump(&mut app, |_| source.count() == 2);
	source.release::<Hud>("en", 0);
	source.release::<Panel>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Panel>());
	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.prepare_locale("es");
	pump(&mut app, |_| source.count() == 4);
	source.release::<Hud>("es", 0);
	pump(&mut app, |world| {
		world
			.resource::<Localization<Root, Manual>>()
			.preparation
			.as_ref()
			.unwrap()
			.store
			.get::<Hud>()
			.is_ok()
	});

	drop(panel);
	app.update();
	let state = app.world().resource::<Localization<Root, Manual>>();
	assert_eq!(state.preparation_status(), PreparationStatus::Ready);
	assert!(
		!state
			.preparation
			.as_ref()
			.unwrap()
			.entries
			.contains_key("presentation/panel.ftl")
	);
	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert!(app.world().contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Panel>());

	source.release::<Panel>("es", 0);
	app.update();
	assert!(!app.world().contains_resource::<Panel>());
	drop(hud);
	app.update();
	assert!(!app.world().contains_resource::<Hud>());
}
