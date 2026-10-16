use crate::texts::presentation::Hud as Interface;
use bevy::prelude::*;
use bevy_fluent_typed::Localized;
use bevy_fluent_typed::Msg;
use crate::button;

pub(super) fn spawn(world: &mut World, message: Msg<Interface>) -> (Entity, Entity) {
	let ui_binding = Localized::<Text, _>::from(message.clone());
	let binding = Localized::<Text2d, _>::from(message);
	let ui = world.spawn_scene(bsn! {
		Localized::<Text, Interface>::from(ui_binding)
	}).unwrap().id();
	let world_label = world.spawn_scene(bsn! {
		Localized::<Text2d, Interface>::from(binding)
		Text2d
	}).unwrap().id();
	(ui, world_label)
}

pub(super) fn spawn_buttons(world: &mut World, message: Msg<Interface>) -> (Entity, [Entity; 2]) {
	let first = message.clone();
	let second = message;
	let root = world.spawn_scene(bsn! {
		Node
		Children [
			#FirstButton button(first),
			#SecondButton button(second)
		]
	}).unwrap().id();
	let children = world.get::<Children>(root).unwrap();
	(root, [children[0], children[1]])
}

pub(super) fn list_caption(message: Msg<Interface>) -> Box<dyn SceneList> {
	Box::new(bsn_list! {
		Text
		Localized::<Text, Interface>::from(message),
		Node
	})
}
