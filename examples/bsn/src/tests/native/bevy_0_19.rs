use crate::texts::presentation::Hud as Interface;
use bevy::{ecs::template::template, prelude::*};
use bevy_fluent_typed::LocalizedText;

pub(super) fn spawn(world: &mut World, binding: LocalizedText<Interface>) -> (Entity, Entity) {
	let ui_binding = binding.clone();
	let ui = world.spawn_scene(bsn! {
		template(move |_| Ok(ui_binding.clone()))
	}).unwrap().id();
	let world_label = world.spawn_scene(bsn! {
		template(move |_| Ok(binding.clone()))
		Text2d
	}).unwrap().id();
	(ui, world_label)
}
