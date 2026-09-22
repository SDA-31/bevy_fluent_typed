//! Standard Bevy source registration; no archive or localization-specific I/O trait.
use crate::texts;
use bevy_fluent_typed::{
	CatalogUpdate, CatalogUpdateReader, FluentCatalog, LocalizationPlugin, LocalizationSystems,
	LocalizedText,
	bevy::{
		asset::io::{
			AssetSourceBuilder,
			memory::{Dir, MemoryAssetReader},
		},
		ecs as bevy_ecs,
		prelude::*,
	},
};
use std::{
	path::Path,
	time::{Duration, Instant},
};

// Headless demonstration timeout and polling interval, not runtime policies.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Resource, Default)]
pub(super) struct Outcomes {
	pub loaded: Vec<texts::Locale>,
	pub rejected: Vec<Option<texts::Locale>>,
	pub errors: Vec<String>,
}

pub(super) fn source_files() -> Dir {
	let files = Dir::default();
	files.insert_asset_text(Path::new(texts::CATALOG_ASSET_PATH), texts::CATALOG_CONFIG);

	// Supply deterministic in-memory data without choosing an archive format.
	// Source files are still required by build.rs to generate the typed API.
	for (locale, path, source) in texts::MODULES {
		files.insert_asset_text(
			&Path::new("localizations")
				.join(texts::LANGUAGES_DIRECTORY)
				.join(locale)
				.join(path),
			source,
		);
	}

	files
}

pub(super) fn app(files: Dir) -> App {
	let mut app = App::new();
	// Register before AssetPlugin builds the source registry. Substitute your
	// archive plugin's AssetReader here; the localization API below stays the same.
	app.register_asset_source(
		"translations",
		AssetSourceBuilder::new(move || {
			Box::new(MemoryAssetReader {
				root: files.clone(),
			})
		}),
	)
	.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			watch_for_changes_override: Some(false),
			..default()
		},
	))
	.add_plugins(LocalizationPlugin::<texts::Translations>::new(
		"translations://localizations/localization.toml",
	))
	.init_resource::<Outcomes>()
	.add_systems(PreUpdate, record.after(LocalizationSystems::Publish));
	app.world_mut().spawn((
		Text::default(),
		LocalizedText::<texts::Translations>::new(|catalog| catalog.ui().hud().msg_title()),
	));
	app.world_mut().spawn((
		Text2d::default(),
		LocalizedText::<texts::Translations>::new(|catalog| catalog.ui().panel().msg_title()),
	));
	app.finish();
	app.cleanup();
	app
}

fn record(mut events: CatalogUpdateReader<texts::Translations>, mut outcomes: ResMut<Outcomes>) {
	for event in events.read() {
		match event {
			CatalogUpdate::Loaded { locale } => outcomes.loaded.push(*locale),
			CatalogUpdate::Rejected { locale, error, .. } => {
				outcomes.rejected.push(*locale);
				outcomes.errors.push(error.clone());
			}
		}
	}
}

pub(super) fn wait_for_load(app: &mut App) {
	let deadline = Instant::now() + LOAD_TIMEOUT;

	loop {
		app.update();
		let outcomes = app.world().resource::<Outcomes>();

		if outcomes.rejected.contains(&None)
			|| outcomes.loaded.len() + outcomes.rejected.len()
				== texts::Translations::locales().len()
		{
			return;
		}

		assert!(
			Instant::now() < deadline,
			"catalog load timed out: {:?}",
			outcomes.errors
		);
		std::thread::sleep(POLL_INTERVAL);
	}
}
