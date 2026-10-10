use crate::texts::presentation::Hud as Interface;
use bevy::prelude::*;
use bevy_fluent_typed::LocalizedText;

pub(super) fn spawn(world: &mut World, binding: LocalizedText<Interface>) -> (Entity, Entity) {
	let ui_binding = binding.clone();
	let ui = world.spawn_scene(bsn! { ui_binding }).unwrap().id();
	let world_label = world.spawn_scene(bsn! { binding Text2d }).unwrap().id();
	(ui, world_label)
}
