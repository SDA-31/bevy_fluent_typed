use super::{Hud, Other, Panel, Presentation, Root, configured_mode, pump};
use crate::bevy::prelude::*;
use crate::{Auto, Localization, LocalizationProgress, LocalizationProgressPlugin, LocalizedText};
use std::sync::Arc;

#[test]
fn spans_leases_and_pins_share_recursive_progress_through_automatic_switching() {
	let (mut app, source) = configured_mode::<Auto>();
	app.add_plugins(LocalizationProgressPlugin::<Root>::new());
	app.finish();
	app.cleanup();
	app.update();
	assert_eq!(source.count(), 0);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Root>>()
			.active()
			.total,
		0
	);

	let parent = app.world_mut().spawn(Text::default()).id();
	let span = app
		.world_mut()
		.spawn((
			TextSpan::default(),
			ChildOf(parent),
			LocalizedText::<Hud>::new(|hud| hud.0.to_string()),
		))
		.id();
	let lease = app
		.world_mut()
		.resource_mut::<Localization<Root>>()
		.hold::<Presentation>();
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.load::<Other>();
	pump(&mut app, |_| source.count() == 3);
	let english = source.release::<Hud>("en", 0);
	source.release::<Panel>("en", 0);
	source.release::<Other>("en", 0);
	pump(&mut app, |world| {
		world
			.resource::<LocalizationProgress<Root>>()
			.active()
			.ready == 3
	});
	let old = Arc::downgrade(&app.world().resource::<Hud>().0);

	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.set_locale("es");
	pump(&mut app, |_| source.count() == 6);
	let spanish = source.release::<Hud>("es", 0);
	pump(&mut app, |world| {
		world
			.resource::<LocalizationProgress<Hud>>()
			.preparation()
			.is_some_and(|target| target.ready == 1)
	});
	assert_eq!(app.world().get::<TextSpan>(span).unwrap().0, english);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Root>>()
			.active()
			.locale,
		"en"
	);

	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.unload::<Other>();
	app.update();
	source.release::<Panel>("es", 0);
	pump(&mut app, |world| {
		world.resource::<Localization<Root>>().locale() == "es"
	});
	assert_eq!(app.world().get::<TextSpan>(span).unwrap().0, spanish);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Root>>()
			.active()
			.total,
		2
	);
	assert!(old.upgrade().is_none());
	assert!(!app.world().contains_resource::<Other>());
	assert!(app.world().get::<Text>(span).is_none());
	assert!(app.world().get::<Text2d>(span).is_none());

	source.release::<Other>("es", 0);
	drop(lease);
	app.update();
	assert!(app.world().contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Panel>());
	assert!(!app.world().contains_resource::<Other>());
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Root>>()
			.active()
			.total,
		1
	);
	let current = Arc::downgrade(&app.world().resource::<Hud>().0);

	app.world_mut().despawn(span);
	app.update();
	assert!(!app.world().contains_resource::<Hud>());
	assert!(current.upgrade().is_none());
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Root>>()
			.active()
			.total,
		0
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.active()
			.unloaded,
		1
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Presentation>>()
			.active()
			.unloaded,
		2
	);
	assert_eq!(source.count(), 6);
}
