use super::{Hud, Panel, Presentation, Root, controlled_mode, pump};
use crate::bevy::prelude::*;
use crate::{Auto, Localization, LocalizedText};

#[test]
fn overlapping_manual_pins_and_automatic_consumers_release_independently() {
	let (mut app, source) = controlled_mode::<Auto>();
	let binding = app
		.world_mut()
		.spawn(LocalizedText::<Hud>::new(|hud| hud.0.to_string()))
		.id();
	{
		let mut state = app.world_mut().resource_mut::<Localization<Root>>();
		state.load::<Presentation>();
		state.load::<Hud>();
		state.load::<Hud>();
	}

	pump(&mut app, |_| source.count() == 2);
	source.release::<Hud>("en", 0);
	source.release::<Panel>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Presentation>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.unload::<Presentation>();
	app.update();
	assert!(app.world().contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Panel>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.unload::<Hud>();
	app.update();
	assert!(app.world().contains_resource::<Hud>());
	app.world_mut().despawn(binding);
	app.update();
	assert!(!app.world().contains_resource::<Hud>());
	assert_eq!(source.count(), 2);
}

#[test]
fn switching_a_partial_tree_tracks_added_and_removed_manual_pins() {
	let (mut app, source) = controlled_mode::<Auto>();
	let binding = app
		.world_mut()
		.spawn(LocalizedText::<Hud>::new(|hud| hud.0.to_string()))
		.id();
	pump(&mut app, |_| source.count() == 1);
	let english = source.release::<Hud>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Root>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.set_locale("es");
	pump(&mut app, |_| source.count() == 2);
	assert_eq!(&**app.world().resource::<Hud>().0, english);
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.load::<Panel>();
	pump(&mut app, |_| source.count() == 4);
	let spanish = source.release::<Hud>("es", 0);
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world().resource::<Localization<Root>>().locale(), "en");
	assert_eq!(&**app.world().resource::<Hud>().0, english);
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.unload::<Panel>();
	pump(&mut app, |world| {
		world.resource::<Localization<Root>>().locale() == "es"
	});
	assert_eq!(&**app.world().resource::<Hud>().0, spanish);
	assert_eq!(app.world().get::<Text>(binding).unwrap().0, spanish);
	assert!(!app.world().contains_resource::<Panel>());
	source.release::<Panel>("en", 0);
	source.release::<Panel>("es", 0);
	for _ in 0..5 {
		app.update();
	}
	assert!(!app.world().contains_resource::<Panel>());
	assert_eq!(&**app.world().resource::<Hud>().0, spanish);
}

#[test]
fn switching_tracks_automatic_demand_added_and_removed_during_preparation() {
	let (mut app, source) = controlled_mode::<Auto>();
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.load::<Hud>();
	pump(&mut app, |_| source.count() == 1);
	source.release::<Hud>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Hud>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.set_locale("es");
	pump(&mut app, |_| source.count() == 2);
	let binding = app
		.world_mut()
		.spawn(LocalizedText::<Panel>::new(|panel| panel.0.to_string()))
		.id();
	pump(&mut app, |_| source.count() == 4);
	let spanish = source.release::<Hud>("es", 0);
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world().resource::<Localization<Root>>().locale(), "en");
	app.world_mut().despawn(binding);
	pump(&mut app, |world| {
		world.resource::<Localization<Root>>().locale() == "es"
	});
	assert_eq!(&**app.world().resource::<Hud>().0, spanish);
	assert!(!app.world().contains_resource::<Panel>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.unload::<Hud>();
	app.update();
	assert!(!app.world().contains_resource::<Hud>());
}
