//! Self-contained virtual files; a real translation pack supplies its own bytes.
use crate::texts;
use bevy_fluent_typed::bevy::asset::io::memory::Dir;
use std::path::Path;

pub(super) fn files() -> Dir {
	let files = Dir::default();
	files.insert_asset_text(Path::new(texts::CATALOG_ASSET_PATH), texts::CATALOG_CONFIG);

	// Reuse build-time sources only to avoid adding an archive dependency to this example.
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
