pub(crate) mod targets;

use crate::texts::ui::Greeting;
use bevy_fluent_typed::bevy::{
	prelude::{
		AccessibleLabel, ChildOf, Color, Commands, Entity, Node, Resource, Text, Text2d, TextColor,
		TextSpan,
	},
	ui_widgets::Button,
};
use bevy_fluent_typed::{Localized, Msg};
use targets::{Badge, Caption, CaptionMirror, Card, CardDetail, CardTitle};

#[derive(Resource)]
pub(crate) struct Widgets {
	pub ui: Entity,
	pub world: Entity,
	pub span: Entity,
	pub button: Entity,
	pub button_text: Entity,
	pub caption: Entity,
	pub badge: Entity,
	pub card: Entity,
}

pub(crate) fn spawn_widgets(mut commands: Commands) {
	let message = pilot("Ada");

	// Explicit Text preserves the application's style and other components.
	let ui = commands
		.spawn((
			Text::default(),
			TextColor(Color::srgb(0.2, 0.8, 0.9)),
			Localized::<Text, Greeting>::from(message.clone()),
		))
		.id();

	let world = commands
		.spawn((
			Text2d::default(),
			Localized::<Text2d, Greeting>::from(message.clone()),
		))
		.id();
	let span = commands
		.spawn((
			ChildOf(world),
			TextSpan::default(),
			Localized::<TextSpan, Greeting>::from(message.clone()),
		))
		.id();

	// Button behavior and layout stay in Bevy. Name and visible caption are explicit.
	let button_text = commands
		.spawn((
			Text::default(),
			Localized::<Text, Greeting>::from(message.clone()),
		))
		.id();
	let button = commands
		.spawn((
			Button,
			Node::default(),
			AccessibleLabel::default(),
			Localized::<AccessibleLabel, Greeting>::from(message.clone()),
		))
		.add_child(button_text)
		.id();

	// Custom components use the same companion binding as native text.
	let caption = commands
		.spawn((
			Caption::default(),
			CaptionMirror::default(),
			Localized::<Caption, _>::from(message.clone()),
		))
		.id();

	// A single-field component uses the derive, through the same target engine.
	let badge = commands
		.spawn((
			Badge::default(),
			Localized::<Badge, _>::from(message.clone()),
		))
		.id();

	// Card has no Default: the application supplies it, the bindings update fields.
	let card = commands
		.spawn((
			Card {
				title: String::new(),
				detail: String::new(),
			},
			Localized::<CardTitle, Greeting>::new(|greeting| greeting.msg_title()),
			Localized::<CardDetail, _>::from(message),
		))
		.id();

	commands.insert_resource(Widgets {
		ui,
		world,
		span,
		button,
		button_text,
		caption,
		badge,
		card,
	});
}

pub(crate) fn pilot(name: &str) -> Msg<Greeting> {
	let name = name.to_owned();
	Msg::new(move |greeting: &Greeting| greeting.msg_detail(&name))
}
