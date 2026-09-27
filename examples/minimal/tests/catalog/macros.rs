use localization_runtime::LocalizationManifest;

mod fixture {
	use localization_runtime::translations;

	// Caller scope must not override the macro's defining runtime dependency.
	mod bevy_fluent_codegen_bridge {}

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
