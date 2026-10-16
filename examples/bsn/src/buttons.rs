use bevy::{
	prelude::{AccessibleLabel, Children, Scene, SceneList, Text, bsn},
	ui_widgets::Button,
};
use bevy_fluent_typed::{FluentScope, Localized, Msg};

#[path = "buttons/native/bevy_0_20.rs"]
mod native;

pub use native::caption_button;

/// Put visible text on the child and its accessible name on the button root.
pub fn button<S: FluentScope>(message: Msg<S>) -> impl Scene {
	let accessible = message.clone();

	bsn! {
		Button
		AccessibleLabel
		Localized::<AccessibleLabel, S>::from(accessible)
		Children [
			Text
			Localized::<Text, S>::from(message)
		]
	}
}

/// Accept multiple caption entities, matching Feathers' SceneList slot.
pub fn caption_list_button<S: FluentScope>(
	caption: Box<dyn SceneList>,
	name: Msg<S>,
) -> impl Scene {
	bsn! {
		Button
		AccessibleLabel
		Localized::<AccessibleLabel, S>::from(name)
		Children [ {caption} ]
	}
}
