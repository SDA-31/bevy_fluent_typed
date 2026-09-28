use localization_runtime::LocalizationManifest;

mod fixture {
	use localization_runtime::translations;

	// Caller scope must not override the macro's defining runtime dependency.
	mod bevy_fluent_typed {}

	translations!(
		/// A nested module with restricted visibility and an optional semicolon.
		pub(super) mod texts;
	);

	// Forwarded attributes must apply to the whole generated module.
	translations!(
		#[cfg(any())]
		mod texts
	);
}

#[test]
fn macro_resolves_the_renamed_runtime_and_consumer_build_output() {
	let translated: fixture::texts::Translations = fixture::texts::Translations::from_manifest(
		fixture::texts::Locale::En,
		&fixture::texts::embed_manifest!(),
	)
	.unwrap();
	let hud: &fixture::texts::presentation::Hud = translated.presentation().hud();
	let expected = super::load(crate::texts::Locale::En);

	assert_eq!(
		translated.ui().msg_example_greeting("Ada"),
		expected.ui().msg_example_greeting("Ada")
	);
	assert_eq!(hud.prompt().s0, expected.presentation().hud().prompt().s0);
	assert_eq!(
		LocalizationManifest::parse(fixture::texts::CATALOG_CONFIG, "catalog.toml")
			.unwrap()
			.config()
			.languages_directory,
		LocalizationManifest::parse(crate::texts::CATALOG_CONFIG, "catalog.toml")
			.unwrap()
			.config()
			.languages_directory
	);
	assert_eq!(fixture::texts::MODULES, crate::texts::MODULES);
}

#[test]
fn generated_manifest_uses_build_configuration_without_loading_sources() {
	let manifest = fixture::texts::manifest();

	assert_eq!(
		manifest.file_path(),
		Some(std::path::Path::new(fixture::texts::CATALOG_ASSET_PATH))
	);
	assert_eq!(
		manifest.config().source_language,
		fixture::texts::SOURCE_LANGUAGE
	);
	assert_eq!(
		manifest.config().default_language,
		fixture::texts::DEFAULT_LANGUAGE
	);
	assert_eq!(
		manifest.config().languages_directory,
		std::path::Path::new(fixture::texts::LANGUAGES_DIRECTORY)
	);
	assert!(manifest.embedded_modules().is_none());
}

#[test]
fn typed_embedding_resolves_nested_modules_and_import_aliases() {
	use fixture::texts::presentation::Hud as FlightHud;

	let selected = fixture::texts::embed_manifest!(module = FlightHud);
	let direct = fixture::texts::embed_manifest!(module = fixture::texts::presentation::Hud);
	assert_eq!(selected.embedded_modules(), direct.embedded_modules());
	assert!(
		selected
			.embedded_modules()
			.unwrap()
			.iter()
			.all(|(_, path, _)| *path == "presentation/hud.ftl")
	);
	let hud = FlightHud::from_manifest(fixture::texts::Locale::En, &selected).unwrap();
	assert_eq!(hud.msg_title(), "Flight HUD");

	let group = fixture::texts::embed_manifest!(module = fixture::texts::Presentation);
	assert!(
		group
			.embedded_modules()
			.unwrap()
			.iter()
			.all(|(_, path, _)| path.starts_with("presentation/"))
	);
	let group_hud = FlightHud::from_manifest(fixture::texts::Locale::En, &group).unwrap();
	fixture::texts::presentation::Panel::from_manifest(fixture::texts::Locale::En, &group).unwrap();
	assert_eq!(group_hud.msg_title(), hud.msg_title());
	assert!(group.read("en", "ui.ftl").is_err());
}
