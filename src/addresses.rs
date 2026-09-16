//! Asset-address safety for arbitrary providers; no filesystem discovery policy.
use std::{
	io,
	path::{Component, Path},
};

pub(super) fn validate_directory(path: &Path) -> Result<(), io::Error> {
	let Some(text) = path.to_str() else {
		return Err(io::Error::other("catalog asset directory must be UTF-8"));
	};

	if text.is_empty()
		|| text.contains(['\\', '#', ':'])
		|| path
			.components()
			.any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
	{
		return Err(io::Error::other(format!(
			"invalid relative catalog asset address: {text:?}"
		)));
	}

	Ok(())
}

pub(super) fn validate_locale(locale: &str) -> Result<(), io::Error> {
	validate_directory(Path::new(locale))?;

	if locale.contains('/') || locale == "." {
		return Err(io::Error::other(
			"catalog locale must be one directory name",
		));
	}

	Ok(())
}

pub(super) fn validate_module(module: &str) -> Result<(), io::Error> {
	validate_directory(Path::new(module))?;

	if module.ends_with('/') || Path::new(module).file_name().is_none() {
		return Err(io::Error::other("catalog module must name a file"));
	}

	Ok(())
}

#[cfg(test)]
mod tests {
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
}
