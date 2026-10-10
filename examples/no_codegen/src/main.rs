mod texts;

use bevy_fluent_typed::bevy::prelude::*;
use bevy_fluent_typed::{Localization, LocalizationPlugin, LocalizedText};
use texts::Texts;

fn main() {
	// This example explicitly embeds its own readable bytes; no TOML or generator.
	let plugin = LocalizationPlugin::<Texts>::from_bytes([
		(
			"en",
			"ui/greeting.ftl",
			include_bytes!("../assets/localizations/translations/en/ui/greeting.ftl").as_slice(),
		),
		(
			"es",
			"ui/greeting.ftl",
			include_bytes!("../assets/localizations/translations/es/ui/greeting.ftl").as_slice(),
		),
		(
			"ru",
			"ui/greeting.ftl",
			include_bytes!("../assets/localizations/translations/ru/ui/greeting.ftl").as_slice(),
		),
	])
	.expect("known locale/module keys");
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	let label = app
		.world_mut()
		.spawn((
			Text::default(),
			LocalizedText::<Texts>::new(|texts| texts.hello.clone()),
		))
		.id();
	app.finish();
	app.cleanup();

	// The plugin parses requested bytes and publishes checked resources.
	for locale in ["en", "es", "ru"] {
		app.world_mut()
			.resource_mut::<Localization<Texts>>()
			.set_locale(locale);
		let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);

		loop {
			app.update();
			let localization = app.world().resource::<Localization<Texts>>();

			if localization.locale() == locale && localization.catalog().is_some() {
				break;
			}

			assert!(
				std::time::Instant::now() < deadline,
				"example translation load timed out"
			);
			std::thread::sleep(std::time::Duration::from_millis(5));
		}

		println!("{}", app.world().get::<Text>(label).unwrap().0);
	}
}
