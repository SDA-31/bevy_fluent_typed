use super::{validate_directory, validate_locale, validate_module};
use std::path::Path;

#[test]
fn provider_addresses_cannot_escape_or_switch_asset_sources() {
	assert!(validate_directory(Path::new(".")).is_ok());
	assert!(validate_directory(Path::new("translations/nested")).is_ok());
	assert!(validate_locale("pt-BR").is_ok());
	assert!(validate_module("ui/main.ftl").is_ok());

	for invalid in [
		"",
		"../other",
		"/absolute",
		"other://file",
		"file#label",
		"folder\\file",
	] {
		assert!(validate_directory(Path::new(invalid)).is_err(), "{invalid}");
	}

	for invalid in [".", "en/other", "en/", "../en"] {
		assert!(validate_locale(invalid).is_err(), "{invalid}");
	}

	assert!(validate_module(".").is_err());
	assert!(validate_module("ui/").is_err());
}
