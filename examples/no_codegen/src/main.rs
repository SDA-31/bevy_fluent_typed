mod texts;

use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{Localization, LocalizationPlugin, LocalizedText};
use texts::Texts;

fn main() {
	for greeting in greetings() {
		println!("{greeting}");
	}
}

fn greetings() -> Vec<String> {
	let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: assets.to_string_lossy().into_owned(),
			..default()
		},
		LocalizationPlugin::<Texts>::new("localizations/localization.toml"),
	));
	let label = app
		.world_mut()
		.spawn((
			Text::default(),
			LocalizedText::<Texts>::new(|texts| texts.hello.clone()),
		))
		.id();
	app.finish();
	app.cleanup();

	let mut greetings = Vec::new();

	for locale in ["en", "es", "ru"] {
		app.world_mut()
			.resource_mut::<Localization<Texts>>()
			.set_locale(locale);
		app.update();
		greetings.push(app.world().get::<Text>(label).unwrap().0.clone());
	}

	greetings
}

#[test]
fn handwritten_provider_updates_text_bindings() {
	assert_eq!(greetings(), ["Hello!", "¡Hola!", "Привет!"]);
}
