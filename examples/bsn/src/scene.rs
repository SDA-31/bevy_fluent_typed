use crate::texts::presentation::Hud as Interface;
use bevy::prelude::*;
use bevy_fluent_typed::Localized;

pub fn greeting_scene(name: String) -> impl Scene {
	bsn! {
		Text
		Localized::<Text, Interface>::new(move |hud| hud.msg_hello(&name))
	}
}
