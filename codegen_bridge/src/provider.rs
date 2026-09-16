//! Runtime trait implementation for the generated immutable translation tree.
use fluent_typed_codegen::syn::{Item, parse_quote};

pub(super) fn implementation() -> Item {
	parse_quote! {
		impl __fluent_runtime::FluentCatalog for Translations {
			type Locale = Locale;

			fn locales() -> &'static [Locale] {
				Locale::iter().as_slice()
			}

			fn default_locale() -> Locale {
				DEFAULT_LANGUAGE
					.parse()
					.expect("build-validated default locale")
			}

			fn descriptor(
				definition: &[u8],
			) -> ::std::result::Result<__fluent_runtime::CatalogDescriptor, ::std::string::String> {
				__fluent_bridge::validate_definition(definition, &[
					("translations-directory", LANGUAGES_DIRECTORY),
					("source-language", SOURCE_LANGUAGE),
					("default-language", DEFAULT_LANGUAGE),
				])?;

				::std::result::Result::Ok(__fluent_runtime::CatalogDescriptor {
					modules_directory: LANGUAGES_DIRECTORY.into(),
				})
			}

			fn embedded(locale: Locale) -> Self {
				Self::embedded(locale)
			}

			fn modules(locale: Locale) -> ::std::vec::Vec<__fluent_runtime::Module> {
				let code: &str = locale.as_ref();

				MODULES
					.iter()
					.filter_map(|&(language, path, embedded)| {
						(language == code).then_some(__fluent_runtime::Module { path, embedded })
					})
					.collect()
			}

			fn parse(
				locale: Locale,
				sources: &[__fluent_runtime::ModuleSource<'_>],
			) -> ::std::result::Result<Self, ::std::string::String> {
				let modules: ::std::vec::Vec<_> = sources
					.iter()
					.map(|module| (module.path, module.source))
					.collect();

				Self::from_modules(locale, &modules)
			}

			fn publish_resources(&self, world: &mut __fluent_runtime::bevy::prelude::World) {
				self.__publish(world);
			}
		}
	}
}
