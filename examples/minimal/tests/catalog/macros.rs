use localization_runtime::FluentCatalog;

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
	let translated: fixture::texts::Translations = fixture::texts::Locale::En.load();
	let hud: &fixture::texts::presentation::Hud = translated.presentation().hud();
	let expected = crate::texts::Locale::En.load();

	assert_eq!(
		translated.ui().msg_example_greeting("Ada"),
		expected.ui().msg_example_greeting("Ada")
	);
	assert_eq!(hud.prompt().s0, expected.presentation().hud().prompt().s0);
	assert_eq!(
		fixture::texts::Translations::descriptor(fixture::texts::CATALOG_CONFIG.as_bytes())
			.unwrap()
			.modules_directory,
		crate::texts::Translations::descriptor(crate::texts::CATALOG_CONFIG.as_bytes())
			.unwrap()
			.modules_directory
	);
	assert_eq!(fixture::texts::MODULES, crate::texts::MODULES);
}
