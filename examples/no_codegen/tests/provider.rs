#[path = "../src/texts.rs"]
mod texts;

use bevy_fluent_typed::{FluentCatalog, FluentScope};
use texts::Texts;

#[test]
fn every_language_supplies_our_api() {
	for (locale, expected) in [("en", "Hello!"), ("es", "¡Hola!"), ("ru", "Привет!")] {
		let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("assets/localizations/translations")
			.join(locale)
			.join("ui/greeting.ftl");
		assert_eq!(
			Texts::parse(locale, &std::fs::read(path).unwrap())
				.unwrap()
				.hello,
			expected
		);
	}
	assert_eq!(Texts::locales(), &["en", "es", "ru"]);
	assert_eq!(Texts::module_paths(), &["ui/greeting.ftl"]);
}

#[test]
fn incompatible_candidates_fail_before_publication() {
	for source in [
		"hello = { $missing }",
		"hello = { missing }",
		"other = Hello!",
		"hello = {",
		"",
	] {
		assert!(Texts::parse("en", source.as_bytes()).is_err());
	}
}
