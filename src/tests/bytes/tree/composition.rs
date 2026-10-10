use super::{Hud, Other, Panel, Root, controlled_mode, pump};
use crate::bevy::prelude::*;
use crate::{Lazy, Localization, LocalizedText, PreparationStatus};

#[test]
fn root_removal_spans_pins_and_locale_switch_share_one_demand_union() {
	let (mut app, source) = controlled_mode::<Lazy>();
	let parent = app
		.world_mut()
		.spawn((
			Text::default(),
			LocalizedText::<Root>::new(|root| root.presentation.hud.0.to_string()),
		))
		.id();
	let span = app
		.world_mut()
		.spawn((
			TextSpan::default(),
			ChildOf(parent),
			LocalizedText::<Hud>::new(|hud| hud.0.to_string()),
		))
		.id();
	pump(&mut app, |_| source.count() == 3);
	let english = source.release::<Hud>("en", 0);
	pump(&mut app, |world| {
		world.get::<TextSpan>(span).unwrap().0 == english
	});
	assert!(!app.world().contains_resource::<Root>());
	app.world_mut()
		.entity_mut(parent)
		.remove::<LocalizedText<Root>>();
	app.update();
	assert!(app.world().contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Panel>());
	assert!(!app.world().contains_resource::<Other>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.load::<Other>();
	pump(&mut app, |_| source.count() == 4);
	source.release::<Other>("en", 1);
	pump(&mut app, |world| world.contains_resource::<Other>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.set_locale("es");
	pump(&mut app, |_| source.count() == 6);
	assert_eq!(app.world().get::<TextSpan>(span).unwrap().0, english);
	assert_eq!(
		app.world()
			.resource::<Localization<Root>>()
			.prepared_locale(),
		Some("es")
	);
	#[cfg(feature = "diagnostics")]
	{
		let progress = app
			.world()
			.resource::<Localization<Root>>()
			.progress::<Root>();
		assert_eq!(progress.locale, "en");
		assert_eq!(progress.total, 3);
		assert_eq!(progress.ready, 2);
		assert_eq!(progress.available, 2);
		assert_eq!(progress.unloaded, 1);
	}

	let spanish = source.release::<Hud>("es", 0);
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world().get::<TextSpan>(span).unwrap().0, english);
	assert_eq!(
		app.world()
			.resource::<Localization<Root>>()
			.preparation_status(),
		PreparationStatus::Preparing
	);
	source.release::<Other>("es", 0);
	pump(&mut app, |world| {
		world.resource::<Localization<Root>>().locale() == "es"
	});
	assert_eq!(app.world().get::<TextSpan>(span).unwrap().0, spanish);
	assert!(app.world().get::<Text>(span).is_none());
	assert!(app.world().get::<Text2d>(span).is_none());
	#[cfg(feature = "diagnostics")]
	{
		let progress = app
			.world()
			.resource::<Localization<Root>>()
			.progress::<Root>();
		assert_eq!(progress.locale, "es");
		assert_eq!(progress.ready, 2);
		assert_eq!(progress.available, 2);
		assert_eq!(progress.unloaded, 1);
	}

	app.world_mut().despawn(span);
	app.update();
	assert!(!app.world().contains_resource::<Hud>());
	assert!(app.world().contains_resource::<Other>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.unload::<Other>();
	app.update();
	assert!(!app.world().contains_resource::<Other>());
	#[cfg(feature = "diagnostics")]
	{
		let progress = app
			.world()
			.resource::<Localization<Root>>()
			.progress::<Root>();
		assert_eq!(progress.unloaded, 3);
		assert_eq!(progress.available, 0);
	}
}
