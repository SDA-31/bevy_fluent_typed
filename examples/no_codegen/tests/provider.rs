#[path = "../src/texts.rs"]
mod texts;

use bevy_fluent_typed::{FluentCatalog, ModuleSource};
use texts::Texts;

#[test]
fn every_embedded_language_supplies_our_api() {
	for (locale, expected) in [("en", "Hello!"), ("es", "¡Hola!"), ("ru", "Привет!")] {
		assert_eq!(Texts::embedded(locale).hello, expected);
	}
}

#[test]
fn incompatible_candidates_fail_before_publication() {
	for source in [
		"hello = { $missing }",
		"hello = { missing }",
		"other = Hello!",
		"hello = {",
	] {
		assert!(
			Texts::parse(
				"en",
				&[ModuleSource {
					path: "ui/greeting.ftl",
					source
				}]
			)
			.is_err()
		);
	}

	assert!(Texts::parse("en", &[]).is_err());
	assert!(
		Texts::parse(
			"en",
			&[ModuleSource {
				path: "wrong.ftl",
				source: "hello = Hello!"
			}]
		)
		.is_err()
	);
}

#[test]
fn definition_has_an_explicit_small_contract() {
	assert!(Texts::descriptor(include_bytes!("../assets/localizations/localization.toml")).is_ok());
	assert!(Texts::descriptor(b"translations-directory = '../outside'").is_err());
	assert!(Texts::descriptor(b"translations-directory = 'translations'\nextra = true").is_err());
}
