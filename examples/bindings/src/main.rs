//! Native text, a custom component and independent fields use the same binding.
mod scenario;
mod widgets;

use bevy_fluent_typed::LocalizationPlugin;
use bevy_fluent_typed::bevy::{
	asset::AssetPlugin,
	prelude::{App, MinimalPlugins, Startup},
};
use widgets::spawn_widgets;

bevy_fluent_typed::translations!(mod texts);

texts::embed_manifest! {
	const GREETING = ui::Greeting;
}

fn main() -> Result<(), String> {
	let cache_policy = scenario::cache_policy()?;
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<texts::Translations>::new(GREETING).with_cache_policy(cache_policy),
	));
	app.add_systems(Startup, spawn_widgets);

	scenario::run(app)
}
