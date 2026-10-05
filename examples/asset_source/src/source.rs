//! Self-contained virtual files; a real translation pack supplies its own bytes.
use crate::texts;
use bevy_fluent_typed::bevy::asset::io::memory::Dir;
use std::path::Path;

texts::embed_manifest! {
	const EMBEDDED = Translations;
}

pub(super) fn files() -> Dir {
	let files = Dir::default();
	files.insert_asset_text(
		Path::new("localizations/localization.toml"),
		texts::CATALOG_CONFIG,
	);

	// Reuse build-time sources only to avoid adding an archive dependency to this example.
	for (locale, path, source) in EMBEDDED.embedded_modules().unwrap() {
		files.insert_asset_text(
			&Path::new("localizations")
				.join(texts::LANGUAGES_DIRECTORY)
				.join(locale)
				.join(path),
			std::str::from_utf8(source).unwrap(),
		);
	}

	files
}
