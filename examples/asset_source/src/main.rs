//! Load a generated catalog through a named Bevy asset source, without a window.
use bevy_fluent_typed::bevy::{
	app::{AppExit, ScheduleRunnerPlugin},
	asset::io::{AssetSourceBuilder, memory::MemoryAssetReader},
	prelude::*,
};
use bevy_fluent_typed::{
	CatalogUpdate, CatalogUpdateReader, LocalizationAppExt, LocalizationPlugin,
};
use std::time::Duration;

bevy_fluent_typed::translations!(mod texts);

mod source;

fn main() -> AppExit {
	let files = source::files();

	App::new()
		// Register before AssetPlugin. An archive plugin can provide its reader here.
		.register_asset_source(
			"translations",
			AssetSourceBuilder::new(move || {
				Box::new(MemoryAssetReader {
					root: files.clone(),
				})
			}),
		)
		.add_plugins((
			MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))),
			AssetPlugin::default(),
			LocalizationPlugin::<texts::Translations>::new(
				bevy_fluent_typed::LocalizationManifest::parse(
					texts::CATALOG_CONFIG,
					"translations://localizations/localization.toml",
				)
				.expect("example source contract"),
			),
		))
		.add_localized_startup_systems(show_title)
		.add_systems(Update, report_failure)
		.run()
}

fn show_title(hud: Res<texts::ui::Hud>, mut exit: MessageWriter<AppExit>) {
	println!("{}", hud.msg_title());
	exit.write(AppExit::Success);
}

fn report_failure(
	mut updates: CatalogUpdateReader<texts::Translations>,
	mut exit: MessageWriter<AppExit>,
) {
	for update in updates.read() {
		if let CatalogUpdate::Rejected { error, .. } = update {
			eprintln!("{error}");
			exit.write(AppExit::error());
		}
	}
}
