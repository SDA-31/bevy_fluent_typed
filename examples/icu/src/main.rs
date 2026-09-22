//! Headless dependency injection of application-owned ICU4X formatters.
use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{FluentCatalog, Localization, LocalizationPlugin};
use icu_decimal::options::GroupingStrategy;

bevy_fluent_typed::translations!(mod texts);

mod formatting;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(texts::ASSET_ROOT);
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: assets.to_string_lossy().into_owned(),
			..default()
		},
		LocalizationPlugin::<texts::Translations>::new(texts::CATALOG_ASSET_PATH),
	))
	.insert_resource(formatting::NumberFormats::try_new(GroupingStrategy::Auto)?)
	.add_systems(Startup, formatting::spawn_labels);
	app.finish();
	app.cleanup();
	app.update();

	// Reuse the same formatters and text entities across language changes.
	// Embedded catalogs are available immediately; this example does not wait for I/O.
	for &locale in texts::Translations::locales() {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		app.update();
		let labels = app.world().resource::<formatting::Labels>();
		let damage = &app.world().get::<Text>(labels.damage).unwrap().0;
		let chance = &app.world().get::<Text2d>(labels.chance).unwrap().0;
		println!("{locale}: {damage} | {chance}");
	}

	Ok(())
}
