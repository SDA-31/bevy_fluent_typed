//! App setup and entity inspection for formatter regression tests.
use crate::{formatting, texts};
use bevy_fluent_typed::LocalizationPlugin;
use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use icu_decimal::options::GroupingStrategy;

/// Build the app once; embedded catalogs make the example deterministic without waiting on I/O.
pub(super) fn example_app() -> Result<App, Box<dyn std::error::Error>> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<texts::Translations>::new(texts::embed_manifest!()),
	))
	.insert_resource(formatting::NumberFormats::try_new(GroupingStrategy::Auto)?)
	.add_systems(Startup, formatting::spawn_labels);
	app.finish();
	app.cleanup();
	app.update();

	Ok(app)
}

/// Read actual Bevy UI/world text, not strings formatted through a separate demonstration path.
pub(super) fn labels(app: &App) -> (&str, &str) {
	let labels = app.world().resource::<formatting::Labels>();
	let damage = &app.world().get::<Text>(labels.damage).unwrap().0;
	let chance = &app.world().get::<Text2d>(labels.chance).unwrap().0;

	(damage, chance)
}
