use crate::texts::presentation::Hud as Interface;
use bevy::{ecs::template::template, prelude::*};
use bevy_fluent_typed::LocalizedText;

pub fn greeting_scene(name: String) -> impl Scene {
	let greeting = LocalizedText::<Interface>::new(move |hud| hud.msg_hello(&name));

	// This factory form works on both Bevy 0.19 and 0.20 without a Default binding.
	bsn! {
		Text
		template(move |_| Ok(greeting.clone()))
	}
}
