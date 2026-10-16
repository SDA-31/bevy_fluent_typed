use super::{Interface, native, pump, test_app, texts};
use bevy::{a11y::AccessibilityNode, prelude::*, ui_widgets::Button};
use bevy_fluent_typed::{Localization, Localized, Msg};

#[test]
fn button_and_caption_slot_follow_visible_and_accessible_translations() {
	let (mut app, _) = test_app();
	let message = Msg::<Interface>::new(|hud| hud.msg_hello("Ada"));
	let button = app
		.world_mut()
		.spawn_scene(crate::button(message.clone()))
		.unwrap()
		.id();
	let caption = crate::greeting_scene("Ada".into());
	let slotted = app
		.world_mut()
		.spawn_scene(crate::caption_button(caption, message))
		.unwrap()
		.id();
	let children: Vec<_> = [button, slotted]
		.into_iter()
		.map(|entity| app.world().get::<Children>(entity).unwrap()[0])
		.collect();

	for (locale, expected) in [
		(texts::Locale::En, "Hello"),
		(texts::Locale::Es, "Hola"),
		(texts::Locale::Ru, "Привет"),
	] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		pump(&mut app, |world| {
			[button, slotted].iter().all(|entity| {
				world
					.get::<AccessibleLabel>(*entity)
					.is_some_and(|label| label.0.starts_with(expected) && label.0.contains("Ada"))
			})
		});

		for (entity, child) in [button, slotted].into_iter().zip(&children) {
			assert!(app.world().get::<Button>(entity).is_some());
			assert!(app.world().get::<Text>(entity).is_none());
			assert!(
				app.world()
					.get::<Localized<AccessibleLabel, Interface>>(entity)
					.is_some()
			);
			assert_eq!(
				app.world()
					.get::<AccessibilityNode>(entity)
					.unwrap()
					.label(),
				Some(
					app.world()
						.get::<AccessibleLabel>(entity)
						.unwrap()
						.0
						.as_str()
				)
			);
			assert_eq!(
				app.world().get::<Text>(*child).unwrap().0,
				app.world().get::<AccessibleLabel>(entity).unwrap().0
			);
			assert!(
				app.world()
					.get::<Localized<Text, Interface>>(*child)
					.is_some()
			);
		}
	}

	app.world_mut().despawn(button);
	app.update();
	assert!(app.world().contains_resource::<Interface>());
	app.world_mut().despawn(slotted);
	app.update();
	assert!(!app.world().contains_resource::<Interface>());
}

#[test]
fn native_button_sugar_and_scene_list_slots_preserve_deferred_bindings() {
	let (mut app, _) = test_app();
	let message = Msg::<Interface>::new(|hud| hud.msg_hello("Ada"));
	let (root, buttons) = native::spawn_buttons(app.world_mut(), message.clone());
	let caption = native::list_caption(message.clone());
	let slotted = app
		.world_mut()
		.spawn_scene(crate::caption_list_button(caption, message.clone()))
		.unwrap()
		.id();
	let icon_only = app
		.world_mut()
		.spawn_scene(crate::caption_button(bsn! { Node }, message))
		.unwrap()
		.id();

	for (locale, prefix) in [
		(texts::Locale::En, "Hello"),
		(texts::Locale::Es, "Hola"),
		(texts::Locale::Ru, "Привет"),
	] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		pump(&mut app, |world| {
			world
				.get::<AccessibleLabel>(icon_only)
				.is_some_and(|label| label.0.starts_with(prefix))
		});

		for entity in [buttons[0], buttons[1], slotted, icon_only] {
			assert!(
				app.world()
					.get::<AccessibleLabel>(entity)
					.unwrap()
					.0
					.starts_with(prefix)
			);
			assert!(app.world().get::<Text>(entity).is_none());
		}
	}

	assert_eq!(app.world().get::<Children>(slotted).unwrap().len(), 2);
	let icon_child = app.world().get::<Children>(icon_only).unwrap()[0];
	assert!(app.world().get::<Text>(icon_child).is_none());
	app.world_mut().despawn(root);
	app.world_mut().despawn(slotted);
	app.world_mut().despawn(icon_only);
	app.update();
	assert!(!app.world().contains_resource::<Interface>());
}
