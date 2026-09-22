//! Run once without a GPU, or use --watch and edit this example's FTL files.
localization_runtime::translations!(pub mod texts);

mod console;

use localization_runtime::bevy::{asset::AssetPlugin, prelude::*};
use localization_runtime::{
	FluentCatalog, Localization, LocalizationPlugin, LocalizationSystems, LocalizedText,
};
use std::path::Path;
use texts::Translations;

fn main() -> Result<(), String> {
	let watch = std::env::args().any(|argument| argument == "--watch");
	let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(texts::ASSET_ROOT);
	// Unix watcher events can resolve symlinks (notably macOS /var -> /private/var).
	// Give Bevy the same physical root so its prefix comparison remains valid.
	#[cfg(unix)]
	let root = root.canonicalize().map_err(|error| error.to_string())?;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: root.to_string_lossy().into_owned(),
			watch_for_changes_override: Some(watch),
			..default()
		},
	))
	.init_resource::<console::LoadStatus>()
	.add_plugins(LocalizationPlugin::<Translations>::new(
		texts::CATALOG_ASSET_PATH,
	))
	.add_systems(
		PreUpdate,
		console::observe.after(LocalizationSystems::Publish),
	);
	app.finish();
	app.cleanup();
	let label = app
		.world_mut()
		.spawn((
			Text::default(),
			LocalizedText::<Translations>::new(|catalog| catalog.ui().msg_example_greeting("Ada")),
		))
		.id();
	console::run_until_loaded(&mut app, label, watch)?;

	// The same entity and deferred argument survive each language switch.
	for &locale in Translations::locales() {
		app.world_mut()
			.resource_mut::<Localization<Translations>>()
			.set_locale(locale);
		app.update();
		println!("{locale}: {}", app.world().get::<Text>(label).unwrap().0);
	}

	Ok(())
}
