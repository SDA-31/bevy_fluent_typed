//! Run once without a GPU, or use --watch and edit this example's FTL files.
localization_runtime::translations!(pub mod texts);

#[cfg(test)]
mod tests;

use localization_runtime::bevy::{asset::AssetPlugin, ecs as bevy_ecs, prelude::*};
use localization_runtime::{
	CatalogUpdate, CatalogUpdateReader, FluentCatalog, Localization, LocalizationPlugin,
	LocalizationSystems, LocalizedText,
};
use std::{
	path::Path,
	time::{Duration, Instant},
};
use texts::{Locale, Translations};

// This deadline bounds missing/broken external assets in the one-shot example.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Resource, Default)]
struct LoadStatus {
	loaded: Vec<Locale>,
	failed: bool,
}

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
	.init_resource::<LoadStatus>()
	.add_plugins(LocalizationPlugin::<Translations>::new(
		texts::CATALOG_ASSET_PATH,
	))
	.add_systems(PreUpdate, observe.after(LocalizationSystems::Publish));
	app.finish();
	app.cleanup();
	let label = app
		.world_mut()
		.spawn((
			Text::default(),
			LocalizedText::<Translations>::new(|catalog| catalog.ui().msg_example_greeting("Ada")),
		))
		.id();
	let deadline = Instant::now() + LOAD_TIMEOUT;
	let mut previous = String::new();

	loop {
		app.update();
		let text = &app.world().get::<Text>(label).unwrap().0;

		if *text != previous {
			println!("{text}");
			previous.clone_from(text);
		}

		let status = app.world().resource::<LoadStatus>();

		if !watch {
			if status.failed {
				return Err("external catalogs failed; see diagnostic above".into());
			}

			if Translations::locales()
				.iter()
				.all(|locale| status.loaded.contains(locale))
			{
				break;
			}

			if Instant::now() >= deadline {
				return Err("timed out waiting for external catalogs".into());
			}
		}

		std::thread::sleep(POLL_INTERVAL);
	}

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

fn observe(mut updates: CatalogUpdateReader<Translations>, mut status: ResMut<LoadStatus>) {
	for update in updates.read() {
		match update {
			CatalogUpdate::Loaded { locale } => {
				if !status.loaded.contains(locale) {
					status.loaded.push(*locale);
				}
			}
			CatalogUpdate::Rejected { path, error, .. } => {
				eprintln!("{path}: {error}");
				status.failed = true;
			}
		}
	}
}
