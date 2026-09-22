//! A deliberately small handwritten provider: one argument-free Fluent message.
//! More messages/arguments require their own validation; codegen automates that.
use bevy_fluent_typed::fluent_typed::prelude::L10nBundle;
use bevy_fluent_typed::{CatalogDescriptor, FluentCatalog, Module, ModuleSource};

pub(super) struct Texts {
	pub(super) hello: String,
}

impl FluentCatalog for Texts {
	type Locale = &'static str;

	fn locales() -> &'static [Self::Locale] {
		&["en", "es", "ru"]
	}

	fn default_locale() -> Self::Locale {
		"en"
	}

	fn descriptor(definition: &[u8]) -> Result<CatalogDescriptor, String> {
		let source = std::str::from_utf8(definition).map_err(|error| error.to_string())?;
		let document = source
			.parse::<toml_edit::DocumentMut>()
			.map_err(|error| error.to_string())?;

		if document.len() != 1
			|| document
				.get("translations-directory")
				.and_then(|item| item.as_str())
				!= Some("translations")
		{
			return Err("expected only translations-directory = \"translations\"".into());
		}

		Ok(CatalogDescriptor {
			modules_directory: "translations".into(),
		})
	}

	fn embedded(locale: Self::Locale) -> Self {
		let module = Self::modules(locale)[0];
		Self::parse(
			locale,
			&[ModuleSource {
				path: module.path,
				source: module.embedded,
			}],
		)
		.expect("bundled message is validated by this example's tests")
	}

	fn modules(locale: Self::Locale) -> Vec<Module> {
		let embedded = match locale {
			"en" => include_str!("../assets/localizations/translations/en/ui/greeting.ftl"),
			"es" => include_str!("../assets/localizations/translations/es/ui/greeting.ftl"),
			"ru" => include_str!("../assets/localizations/translations/ru/ui/greeting.ftl"),
			_ => panic!("undeclared example locale"),
		};
		vec![Module {
			path: "ui/greeting.ftl",
			embedded,
		}]
	}

	fn parse(locale: Self::Locale, sources: &[ModuleSource<'_>]) -> Result<Self, String> {
		let [source] = sources else {
			return Err("expected exactly one module: ui/greeting.ftl".into());
		};

		if source.path != "ui/greeting.ftl" {
			return Err("expected module ui/greeting.ftl".into());
		}

		let bundle =
			L10nBundle::new(locale, source.source.as_bytes()).map_err(|error| error.to_string())?;
		// Eagerly resolve our entire API with no arguments. Missing messages,
		// variables or unresolved references reject the candidate before publication.
		let hello = bundle
			.msg("hello", None)
			.map_err(|error| error.to_string())?;
		Ok(Self { hello })
	}
}
