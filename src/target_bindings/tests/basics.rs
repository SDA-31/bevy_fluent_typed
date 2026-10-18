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
fn native_bindings_require_the_selected_target_without_creating_components() {
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

	assert!(app.world().get::<Text>(implicit).is_none());
	assert!(
		app.world()
			.get::<Localized<Text, TestCatalog>>(implicit)
			.is_none()
	);
	assert!(app.world().get::<Text>(ui).unwrap().0.is_empty());
	assert!(app.world().get::<Text>(world).is_none());
	assert!(app.world().get::<Text>(late_world).is_none());
	assert!(app.world().get::<Text>(cancelled).is_none());
	app.world_mut().insert_resource(TestCatalog("ready".into()));
	app.update();

	assert_eq!(app.world().get::<Text>(ui).unwrap().0, "ready");
	assert_eq!(app.world().get::<Text2d>(world).unwrap().0, "ready");
	assert!(app.world().get::<Text>(world).is_none());

	assert!(app.world().get::<Text2d>(late_world).unwrap().0.is_empty());
	assert!(
		app.world()
			.get::<Localized<Text2d, TestCatalog>>(late_world)
			.is_none()
	);
}
