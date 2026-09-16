use super::{Locale, Translations};
use localization_runtime::{FluentCatalog, Localization, Message};

mod contracts;
mod macros;
mod resources;

// Raw upstream output remains independently usable without the Bevy adapter.
#[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
mod raw {
	// Required by the generated validator's explicit parent-scope import.
	#[allow(clippy::single_component_path_imports)]
	use fluent_syntax;
	use localization_runtime::fluent_typed;

	include!(concat!(env!("OUT_DIR"), "/translations.rs"));
}

// A same-named caller constant must not collide with upstream's private EN static.
#[allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
mod namespace {
	use localization_runtime as __fluent_runtime;
	use localization_runtime::__codegen as __fluent_bridge;

	include!(concat!(env!("OUT_DIR"), "/bevy_catalog.rs"));

	const EN: &str = "caller-owned symbol";

	#[test]
	fn upstream_locale_symbols_do_not_leak_into_the_include_scope() {
		assert_eq!(EN, "caller-owned symbol");
		assert!(
			Locale::En
				.load()
				.ui()
				.msg_example_greeting("Ada")
				.contains("Ada")
		);
		assert!(MODULES.iter().any(|&(locale, _, _)| locale == "en"));
	}
}

#[test]
fn example_starts_in_english_and_discovers_all_three_languages() {
	assert_eq!(Locale::default(), Locale::En);
	assert_eq!(Translations::default_locale(), Locale::En);
	assert_eq!(Localization::<Translations>::default().locale(), Locale::En);
	assert_eq!(crate::texts::SOURCE_LANGUAGE, "en");
	assert_eq!(crate::texts::DEFAULT_LANGUAGE, "en");

	let languages: Vec<_> = Translations::locales().iter().map(AsRef::as_ref).collect();
	assert_eq!(languages, ["en", "es", "ru"]);
}

#[test]
fn example_paths_come_from_its_generated_configuration() {
	assert_eq!(crate::texts::ASSET_ROOT, "assets");
	assert_eq!(
		crate::texts::CATALOG_ASSET_PATH,
		"localizations/localization.toml"
	);
	assert_eq!(
		Translations::descriptor(crate::texts::CATALOG_CONFIG.as_bytes())
			.unwrap()
			.modules_directory,
		std::path::Path::new("translations")
	);
}

#[test]
fn generated_modules_preserve_paths_and_each_languages_original_source() {
	for (locale, expected) in [
		(
			Locale::En,
			include_str!("../assets/localizations/translations/en/ui.ftl"),
		),
		(
			Locale::Es,
			include_str!("../assets/localizations/translations/es/ui.ftl"),
		),
		(
			Locale::Ru,
			include_str!("../assets/localizations/translations/ru/ui.ftl"),
		),
	] {
		let modules = Translations::modules(locale);

		assert_eq!(modules.len(), 10);
		let ui = modules
			.iter()
			.find(|module| module.path == "ui.ftl")
			.unwrap();
		assert_eq!(ui.embedded, expected);
	}
}

#[test]
fn generated_external_parser_rejects_broken_typed_contracts() {
	for &locale in Translations::locales() {
		for invalid in [
			"",
			"example-greeting = Hello { $unexpected }!\n",
			"example-greeting = Hello {\n",
		] {
			let modules: Vec<_> = Translations::modules(locale)
				.into_iter()
				.map(|module| {
					(
						module.path,
						if module.path == "ui.ftl" {
							invalid
						} else {
							module.embedded
						},
					)
				})
				.collect();
			assert!(
				Translations::from_modules(locale, &modules).is_err(),
				"{locale}: {invalid}"
			);
		}
	}
}

#[test]
fn raw_upstream_output_still_loads_without_the_generated_bevy_adapter() {
	let raw = raw::Locale::En.load();
	let integrated = Translations::embedded(Locale::En);

	assert_eq!(
		raw.ui().msg_example_greeting("Ada"),
		integrated.ui().msg_example_greeting("Ada")
	);
}

#[test]
fn generated_accessors_and_deferred_arguments_work_for_every_discovered_locale() {
	let name = String::from("Ada");
	let message =
		Message::<Translations>::new(move |catalog| catalog.ui().msg_example_greeting(&name));

	for &locale in Translations::locales() {
		let source = Translations::modules(locale)
			.iter()
			.map(|module| (module.path, module.embedded))
			.collect::<Vec<_>>();
		let external = Translations::from_modules(locale, &source).unwrap();
		assert_eq!(
			message.render(&external),
			message.render(&Translations::embedded(locale))
		);
		assert!(message.render(&external).contains("Ada"));
	}
}

#[test]
fn spanish_embedded_and_external_catalogs_preserve_their_own_translations() {
	let sources: Vec<_> = Translations::modules(Locale::Es)
		.into_iter()
		.map(|module| (module.path, module.embedded))
		.collect();
	let external = Translations::from_modules(Locale::Es, &sources).unwrap();

	for translations in [Translations::embedded(Locale::Es), external] {
		assert_eq!(translations.locale(), Locale::Es);
		assert_eq!(translations.catalog().msg_title(), "Catálogo de productos");
		assert_eq!(
			translations.presentation().hud().msg_title(),
			"Panel de vuelo"
		);
		assert_eq!(
			translations.presentation().panel().msg_title(),
			"Panel de ajustes"
		);
		assert!(
			translations
				.ui()
				.msg_example_greeting("Ada")
				.starts_with("¡Hola,")
		);
		assert!(
			translations
				.ui()
				.msg_example_greeting("Ada")
				.contains("Ada")
		);
		assert!(
			translations
				.presentation()
				.hud()
				.prompt()
				.s0
				.contains("Pulsa")
		);
	}
}
