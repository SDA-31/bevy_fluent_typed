//! Validate the generator's immutable definition at the adapter boundary.
use std::path::Path;
use toml_edit::DocumentMut;

/// Implementation detail used by generated providers, not application policy.
///
/// `expected` comes from build-validated generated metadata. Require its field
/// inventory and string values: language codes compare exactly, while directory
/// values use Path equality (accepting redundant separators or a trailing `.`).
/// Comments, whitespace and field order do not affect compatibility. The adapter
/// uses compiled paths after validation, never untrusted paths from edited TOML.
///
/// # Errors
/// Rejects invalid UTF-8/TOML, missing/extra/non-string fields or changed values.
#[doc(hidden)]
pub fn validate_definition(bytes: &[u8], expected: &[(&str, &str)]) -> Result<(), String> {
	let source = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
	let document = source
		.parse::<DocumentMut>()
		.map_err(|error| error.to_string())?;
	let table = document.as_table();

	if table.len() != expected.len()
		|| expected.iter().any(|&(key, value)| {
			let Some(actual) = table.get(key).and_then(|item| item.as_str()) else {
				return true;
			};

			if key == "languages-directory" {
				return Path::new(actual) != Path::new(value);
			}

			actual != value
		}) {
		return Err("localization configuration changed; rebuild required".into());
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::validate_definition;

	#[test]
	fn definitions_accept_formatting_but_reject_changed_contracts() {
		let expected = [
			("languages-directory", "translations"),
			("source-language", "en"),
			("default-language", "es"),
		];
		let valid = "# comment\ndefault-language='es'\nsource-language = 'en'\nlanguages-directory = 'translations'\n";
		assert!(validate_definition(valid.as_bytes(), &expected).is_ok());

		for equivalent in ["translations/", "translations/.", "translations//"] {
			assert!(
				validate_definition(
					valid
						.replace("'translations'", &format!("'{equivalent}'"))
						.as_bytes(),
					&expected
				)
				.is_ok()
			);
		}

		for invalid in [
			valid.replace("'es'", "'ru'"),
			valid.replace("'en'", "7"),
			valid.replace("source-language", "source-langauge"),
			valid.replace("source-language = 'en'\n", ""),
			format!("{valid}\nunknown = 'field'"),
			format!("{valid}\n[extra]"),
			valid.replace("'translations'", "'../outside'"),
			valid.replace("'translations'", "'different'"),
		] {
			assert!(
				validate_definition(invalid.as_bytes(), &expected).is_err(),
				"{invalid}"
			);
		}

		assert!(validate_definition(&[0xff], &expected).is_err());
	}
}
