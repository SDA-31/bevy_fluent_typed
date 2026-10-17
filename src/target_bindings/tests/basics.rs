use crate::bevy::prelude::*;
use crate::tests::TestCatalog;
use crate::{Localized, Msg, bindings};

#[test]
fn deferred_messages_refresh_and_clear_existing_ui_and_world_labels() {
	let message = Msg::<TestCatalog>::new(|catalog| catalog.0.clone());
	let cloned = message.clone();
	let binding = Localized::<Text, _>::from(message.clone());
	let mut app = App::new();
	bindings::register::<TestCatalog>(&mut app, false);
	app.insert_resource(TestCatalog("ja".into()));
	let ui = app
		.world_mut()
		.spawn((Text::default(), binding.clone()))
		.id();
	let world = app
		.world_mut()
		.spawn((Text2d::default(), Localized::<Text2d, _>::from(message)))
		.id();
	app.update();
	assert_eq!(app.world().get::<Text>(ui).unwrap().0, "ja");
	app.world_mut().insert_resource(TestCatalog("es".into()));
	app.update();
	assert_eq!(app.world().get::<Text>(ui).unwrap().0, "es");
	assert_eq!(app.world().get::<Text2d>(world).unwrap().0, "es");
	assert_eq!(cloned.render(app.world().resource()), "es");
	app.world_mut().remove_resource::<TestCatalog>();
	app.update();
	assert!(app.world().get::<Text>(ui).unwrap().0.is_empty());
	assert!(app.world().get::<Text2d>(world).unwrap().0.is_empty());
}

#[test]
fn native_defaults_create_only_the_selected_target_without_a_catalog() {
	let mut app = App::new();
	bindings::register::<TestCatalog>(&mut app, false);
	let binding = Localized::<Text, TestCatalog>::new(|catalog| catalog.0.clone());
	let implicit = app.world_mut().spawn(binding.clone()).id();
	let ui = app
		.world_mut()
		.spawn((binding.clone(), Text::default()))
		.id();
	let world = app
		.world_mut()
		.spawn((
			Localized::<Text2d, TestCatalog>::new(|catalog| catalog.0.clone()),
			Text2d::default(),
		))
		.id();
	let late_world = app
		.world_mut()
		.spawn(Localized::<Text2d, TestCatalog>::new(|catalog| {
			catalog.0.clone()
		}))
		.id();
	app.world_mut()
		.entity_mut(late_world)
		.insert(Text2d::default());
	let cancelled = app.world_mut().spawn(binding.clone()).id();
	app.world_mut()
		.entity_mut(cancelled)
		.remove::<Localized<Text, TestCatalog>>();
	let despawned = app.world_mut().spawn(binding).id();
	app.world_mut().despawn(despawned);
	app.update();

	assert!(app.world().get::<Text>(implicit).unwrap().0.is_empty());
	assert!(app.world().get::<Text>(ui).unwrap().0.is_empty());
	assert!(app.world().get::<Text>(world).is_none());
	assert!(app.world().get::<Text>(late_world).is_none());
	// Initialization may already have run when an exclusive world access flushed hooks.
	assert!(app.world().get::<Text>(cancelled).unwrap().0.is_empty());
	app.world_mut().insert_resource(TestCatalog("ready".into()));
	app.update();

	for entity in [implicit, ui] {
		assert_eq!(app.world().get::<Text>(entity).unwrap().0, "ready");
	}

	for entity in [world, late_world] {
		assert_eq!(app.world().get::<Text2d>(entity).unwrap().0, "ready");
		assert!(app.world().get::<Text>(entity).is_none());
	}
}
