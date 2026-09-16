//! Validate the generator's immutable definition at the adapter boundary.
use std::path::Path;
use toml_edit::{DocumentMut, Table};

/// Implementation detail used by generated providers, not application policy.
///
/// `expected` comes from build-validated generated metadata. Language codes compare
/// exactly; directory values use Path equality (accepting redundant separators or
/// a trailing `.`). An omitted `translations-directory` resolves to `.`. Its legacy
/// alias `languages-directory` is accepted in both input and compiled metadata,
/// but input cannot contain both keys.
/// Comments, whitespace and field order do not affect compatibility. The adapter
/// uses compiled paths after validation, never untrusted paths from edited TOML.
///
/// # Errors
/// Rejects invalid UTF-8/TOML, missing required fields, extra/non-string fields,
/// conflicting directory aliases or changed resolved values.
#[doc(hidden)]
pub fn validate_definition(bytes: &[u8], expected: &[(&str, &str)]) -> Result<(), String> {
	let source = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
	let document = source
		.parse::<DocumentMut>()
		.map_err(|error| error.to_string())?;
	let table = document.as_table();

	if table.contains_key("translations-directory") && table.contains_key("languages-directory") {
		return Err(
			"use only translations-directory; languages-directory is its legacy alias".into(),
		);
	}

	let unknown_field = table.iter().any(|(key, _)| {
		!expected
			.iter()
			.any(|&(field, _)| canonical_field(key) == canonical_field(field))
	});

	if unknown_field
		|| expected.iter().any(|&(key, value)| {
			let Some(actual) = definition_value(table, key) else {
				return true;
			};

			if canonical_field(key) == "translations-directory" {
				return Path::new(actual) != Path::new(value);
			}

			actual != value
		}) {
		return Err("localization configuration changed; rebuild required".into());
	}

	Ok(())
}

fn canonical_field(key: &str) -> &str {
	match key {
		"languages-directory" => "translations-directory",
		_ => key,
	}
}

fn definition_value<'a>(table: &'a Table, key: &str) -> Option<&'a str> {
	if canonical_field(key) == "translations-directory" {
		let Some(item) = table
			.get("translations-directory")
			.or_else(|| table.get("languages-directory"))
		else {
			return Some(".");
		};

		return item.as_str().filter(|value| !value.trim().is_empty());
	}

	table.get(key).and_then(|item| item.as_str())
}

#[cfg(test)]
mod tests {
	use super::validate_definition;

	#[test]
	fn definitions_accept_formatting_but_reject_changed_contracts() {
		let expected = [
			("translations-directory", "translations"),
			("source-language", "en"),
			("default-language", "es"),
		];
		let valid = "# comment\ndefault-language='es'\nsource-language = 'en'\ntranslations-directory = 'translations'\n";
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

	#[test]
	fn directory_aliases_and_omission_compare_resolved_values() {
		let source = "source-language = 'en'\ndefault-language = 'es'\n";

		for expected_key in ["translations-directory", "languages-directory"] {
			for directory in [".", "translations"] {
				let expected = [
					(expected_key, directory),
					("source-language", "en"),
					("default-language", "es"),
				];
				assert_eq!(
					validate_definition(source.as_bytes(), &expected).is_ok(),
					directory == "."
				);

				for key in ["translations-directory", "languages-directory"] {
					let valid = format!("{source}{key} = '{directory}'\n");
					assert!(validate_definition(valid.as_bytes(), &expected).is_ok());

					for invalid in [
						"7",
						"[]",
						"''",
						"' '",
						"'../outside'",
						"'/absolute'",
						"'other'",
					] {
						let source = format!("{source}{key} = {invalid}\n");
						assert!(
							validate_definition(source.as_bytes(), &expected).is_err(),
							"{source}"
						);
					}
				}

				let both = format!(
					"{source}translations-directory = '{directory}'\nlanguages-directory = '{directory}'\n"
				);
				let error = validate_definition(both.as_bytes(), &expected).unwrap_err();
				assert!(error.contains("use only translations-directory"), "{error}");
			}
		}
	}
}
