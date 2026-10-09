//! Typed catalog resources, chained borrows and language-switchable Bevy text.
extern crate localization_runtime as bevy_fluent_typed;

use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{Localization, LocalizationAppExt, LocalizationPlugin, LocalizedText};

bevy_fluent_typed::translations!(pub mod texts);

texts::embed_manifest! {
	const EMBEDDED = Translations;
}

fn main() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.add_plugins(LocalizationPlugin::<texts::Translations>::new(EMBEDDED))
		.add_localized_startup_systems(show_hud);
	app.finish();
	app.cleanup();
	app.update();
	app.update(); // Allow the once-helper to run after automatic publication.

	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Es);
	app.update();

	// Direct resources and existing text bindings follow the same language switch.
	let hud: &texts::presentation::Hud = app.world().resource();
	println!("{}", hud.msg_title());

	let mut labels = app.world_mut().query::<&Text>();
	let label = labels.single(app.world()).expect("one localized label");
	println!("{}", label.0);
}

fn show_hud(mut commands: Commands, translations: Res<texts::Translations>) {
	// This demonstration explicitly consumes the whole tree for chained borrows.
	let presentation: &texts::Presentation = translations.presentation();
	let hud: &texts::presentation::Hud = presentation.hud();
	println!("{}: {}", hud.msg_title(), hud.msg_detail("Ada"));
	println!("{}", hud.msg_title());

	commands.spawn((
		Text::default(),
		LocalizedText::<texts::presentation::Hud>::new(|hud| hud.msg_detail("Ada")),
	));
}
