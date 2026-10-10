//! Headless dependency injection of application-owned ICU4X formatters.
use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{FluentCatalog, Localization, LocalizationPlugin};
use icu_decimal::options::GroupingStrategy;
use std::time::{Duration, Instant};

bevy_fluent_typed::translations!(mod texts);

texts::embed_manifest! {
	const EMBEDDED = Translations;
}

mod formatting;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<texts::Translations>::new(EMBEDDED),
	))
	.insert_resource(formatting::NumberFormats::try_new(GroupingStrategy::Auto)?)
	.add_systems(Startup, formatting::spawn_labels);
	app.finish();
	app.cleanup();
	app.update();

	// Reuse the same formatters and text entities across language changes.
	// Explicit embedded sources are parsed on demand without external I/O.
	for &locale in texts::Translations::locales() {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		let deadline = Instant::now() + Duration::from_secs(10);

		// This headless demo drives Bevy frames manually; normal apps use App::run().
		// Each update lets Bevy publish completed loads and refresh localized text.
		loop {
			app.update();

			if app
				.world()
				.resource::<Localization<texts::Translations>>()
				.locale() == locale
				&& app.world().contains_resource::<texts::presentation::Hud>()
			{
				break;
			}

			assert!(Instant::now() < deadline, "ICU translation load timed out");
			std::thread::sleep(Duration::from_millis(1));
		}
		let labels = app.world().resource::<formatting::Labels>();
		let damage = &app.world().get::<Text>(labels.damage).unwrap().0;
		let chance = &app.world().get::<Text2d>(labels.chance).unwrap().0;
		println!("{locale}: {damage} | {chance}");
	}

	Ok(())
}
