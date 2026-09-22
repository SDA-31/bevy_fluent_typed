mod texts;

use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{Localization, LocalizationPlugin, LocalizedText};
use texts::Texts;

fn main() {
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

	// The same text entity follows each language change using embedded catalogs.
	for locale in ["en", "es", "ru"] {
		app.world_mut()
			.resource_mut::<Localization<Texts>>()
			.set_locale(locale);
		app.update();
		println!("{}", app.world().get::<Text>(label).unwrap().0);
	}
}
