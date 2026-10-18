//! Fixed-language BSN bindings share catalogs and keep their selection across scene conversion.
use super::{Interface, native, pump, reload, test_app, texts};
use bevy::{
	a11y::AccessibilityNode,
	prelude::{AccessibleLabel, Entity, Text, Text2d, TextSpan, World, bsn},
	scene::WorldSceneExt,
};
use bevy_fluent_typed::{Localization, Localized, Msg};

fn starts_with(world: &World, entity: Entity, prefix: &str) -> bool {
	world
		.get::<Text>(entity)
		.is_some_and(|text| text.0.starts_with(prefix))
}

fn fixed_ready(world: &World) -> bool {
	world
		.resource::<Localization<texts::Translations>>()
		.for_locale(texts::Locale::Es)
		.modules()
		.presentation()
		.hud()
		.is_ok()
}

#[test]
fn fixed_bsn_language_and_generated_navigation_share_the_provider() {
	let (mut app, _) = test_app();
	let message = Msg::<Interface>::new(|hud| hud.msg_hello("Ada"));
	let manual = message.clone();
	let entity = app
		.world_mut()
		.spawn_scene(native::fixed_scene(message, texts::Locale::Es))
		.unwrap()
		.id();

	pump(&mut app, |world| starts_with(world, entity, "Hola"));

	let localization = app.world().resource::<Localization<texts::Translations>>();
	let hud = localization
		.for_locale(texts::Locale::Es)
		.modules()
		.presentation()
		.hud()
		.unwrap();

	assert!(manual.render(hud).starts_with("Hola"));
	assert_eq!(localization.locale(), texts::Locale::En);
	assert!(!app.world().contains_resource::<Interface>());
}

#[test]
fn fixed_scene_keeps_its_language_during_switch_reload_and_release() {
	let (mut app, source) = test_app();
	let following = app
		.world_mut()
		.spawn_scene(crate::greeting_scene("Ada".into()))
		.unwrap()
		.id();
	let message = Msg::<Interface>::new(|hud| hud.msg_hello("Ada"));
	let fixed = app
		.world_mut()
		.spawn_scene(native::fixed_scene(message, texts::Locale::Es))
		.unwrap()
		.id();

	pump(&mut app, |world| {
		starts_with(world, following, "Hello") && starts_with(world, fixed, "Hola")
	});

	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Ru);

	pump(&mut app, |world| starts_with(world, following, "Привет"));

	assert!(starts_with(app.world(), fixed, "Hola"));

	source
		.lock()
		.unwrap()
		.insert("es".into(), b"hello = Updated, { $name }!".to_vec());
	reload(&mut app);

	pump(&mut app, |world| starts_with(world, fixed, "Updated"));

	assert!(starts_with(app.world(), following, "Привет"));
	assert!(app.world().get::<Text>(fixed).unwrap().0.contains("Ada"));

	app.world_mut().despawn(fixed);

	pump(&mut app, |world| !fixed_ready(world));

	assert!(starts_with(app.world(), following, "Привет"));
	assert!(app.world().contains_resource::<Interface>());
}

#[test]
fn cloned_fixed_bindings_and_follow_override_survive_scene_conversion() {
	let (mut app, _) = test_app();
	let name = String::from("Ada");
	let binding = Localized::<Text, Interface>::new(move |hud| hud.msg_hello(&name))
		.with_locale(texts::Locale::Es);
	let first_binding = binding.clone();
	let second_binding = binding.clone();
	let first = app
		.world_mut()
		.spawn_scene(bsn! {
			Text
			Localized::<Text, Interface>::from(first_binding)
		})
		.unwrap()
		.id();
	let second = app
		.world_mut()
		.spawn_scene(bsn! {
			Text
			Localized::<Text, Interface>::from(second_binding)
		})
		.unwrap()
		.id();
	let following = app
		.world_mut()
		.spawn_scene(native::following_scene(binding))
		.unwrap()
		.id();

	pump(&mut app, |world| {
		starts_with(world, first, "Hola")
			&& starts_with(world, second, "Hola")
			&& starts_with(world, following, "Hello")
	});

	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Ru);

	pump(&mut app, |world| starts_with(world, following, "Привет"));

	assert!(starts_with(app.world(), first, "Hola"));
	assert!(starts_with(app.world(), second, "Hola"));

	app.world_mut().despawn(first);
	app.update();

	assert!(fixed_ready(app.world()));
	assert!(app.world().get::<Text>(second).unwrap().0.contains("Ada"));

	app.world_mut().despawn(second);

	pump(&mut app, |world| !fixed_ready(world));

	assert!(starts_with(app.world(), following, "Привет"));
}

#[test]
fn bsn_targets_can_select_different_languages_without_changing_the_game_locale() {
	let (mut app, _) = test_app();
	let world_label = Localized::<Text2d, Interface>::new(|hud| hud.msg_hello("Ada"))
		.with_locale(texts::Locale::Es);
	let span_binding = Localized::<TextSpan, Interface>::new(|hud| hud.msg_hello("Ada"))
		.with_locale(texts::Locale::Ru);
	let accessible = Localized::<AccessibleLabel, Interface>::new(|hud| hud.msg_hello("Ada"))
		.with_locale(texts::Locale::Es);
	let parent = app
		.world_mut()
		.spawn_scene(bsn! {
			Text
			Localized::<Text, Interface>::new(|hud| hud.msg_hello("Ada"))
			AccessibleLabel
			Localized::<AccessibleLabel, Interface>::from(accessible)
		})
		.unwrap()
		.id();
	let label = app
		.world_mut()
		.spawn_scene(bsn! {
			Text2d
			Localized::<Text2d, Interface>::from(world_label)
		})
		.unwrap()
		.id();
	let span = app
		.world_mut()
		.spawn_scene(bsn! {
			TextSpan
			Localized::<TextSpan, Interface>::from(span_binding)
		})
		.unwrap()
		.id();

	app.world_mut().entity_mut(parent).add_child(span);

	pump(&mut app, |world| {
		starts_with(world, parent, "Hello")
			&& world
				.get::<Text2d>(label)
				.is_some_and(|text| text.0.starts_with("Hola"))
			&& world
				.get::<TextSpan>(span)
				.is_some_and(|text| text.0.starts_with("Привет"))
			&& world
				.get::<AccessibleLabel>(parent)
				.is_some_and(|text| text.0.starts_with("Hola"))
	});

	let world = app.world();

	assert_eq!(
		world
			.resource::<Localization<texts::Translations>>()
			.locale(),
		texts::Locale::En
	);
	assert_eq!(
		world.get::<AccessibilityNode>(parent).unwrap().label(),
		Some(world.get::<AccessibleLabel>(parent).unwrap().0.as_str())
	);
	assert!(world.get::<Text>(label).is_none());
	assert!(world.get::<Text>(span).is_none());
	assert!(world.get::<Text2d>(span).is_none());
}
