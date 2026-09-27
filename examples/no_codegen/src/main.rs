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
		LocalizationPlugin::<Texts>::new(texts::manifest()),
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

	// Files load asynchronously through AssetServer for the selected language.
	for locale in ["en", "es", "ru"] {
		app.world_mut()
			.resource_mut::<Localization<Texts>>()
			.set_locale(locale);
		let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
		while app
			.world()
			.resource::<Localization<Texts>>()
			.catalog()
			.is_none()
		{
			app.update();
			assert!(
				std::time::Instant::now() < deadline,
				"example translation load timed out"
			);
			std::thread::sleep(std::time::Duration::from_millis(5));
		}
		println!("{}", app.world().get::<Text>(label).unwrap().0);
	}
}
