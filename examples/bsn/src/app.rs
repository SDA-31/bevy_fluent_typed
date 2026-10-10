use crate::texts;
use bevy::{app::ScheduleRunnerPlugin, asset::AssetPlugin, prelude::*, scene::ScenePlugin};
use bevy_fluent_typed::LocalizationPlugin;
use std::time::Duration;

pub fn app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))),
		AssetPlugin {
			file_path: env!("CARGO_MANIFEST_DIR").into(),
			..default()
		},
		ScenePlugin,
		LocalizationPlugin::<texts::Translations>::new(texts::manifest()),
	));
	app
}
