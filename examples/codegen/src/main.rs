use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{Localization, LocalizationPlugin};

bevy_fluent_typed::translations!(mod texts);

fn main() {
	for greeting in greetings() {
		println!("{greeting}");
	}
}

fn greetings() -> Vec<String> {
	let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(texts::ASSET_ROOT);
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: assets.to_string_lossy().into_owned(),
			..default()
		},
		LocalizationPlugin::<texts::Translations>::new(texts::CATALOG_ASSET_PATH),
	));
	app.finish();
	app.cleanup();

	let mut greetings = Vec::new();

	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		app.update();
		let greeting: &texts::ui::Greeting = app.world().resource();
		greetings.push(greeting.msg_hello());
	}

	greetings
}

#[test]
fn generated_resources_switch_languages() {
	assert_eq!(greetings(), ["Hello!", "¡Hola!", "Привет!"]);
}
