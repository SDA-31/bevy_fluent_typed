use bevy::{
	prelude::{AccessibleLabel, Children, Scene, bsn},
	ui_widgets::Button,
};
use bevy_fluent_typed::{FluentScope, Localized, Msg};

/// A caption slot may contain text, an icon, or an application-owned scene.
pub fn caption_button<S: FluentScope>(caption: impl Scene, name: Msg<S>) -> impl Scene {
	bsn! {
		Button
		AccessibleLabel
		Localized::<AccessibleLabel, S>::from(name)
		Children [ @caption ]
	}
}
