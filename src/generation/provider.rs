//! Root provider descriptors and file-source contracts, without FTL payloads.
use fluent_typed_codegen::{
	Scope,
	syn::{Expr, Item, parse_quote},
};

pub(super) fn manifest() -> Item {
	parse_quote! {
		/// Use the file-source contract configured in this application's build metadata.
		///
		/// This uses the validated TOML and its Bevy asset address. It reads no files,
		/// loads no translations and includes no FTL bytes. AssetPlugin still owns
		/// the runtime asset root. Pass a custom LocalizationManifest to the plugin
		/// when runtime storage differs from the build configuration.
		pub fn manifest() -> __fluent_runtime::LocalizationManifest {
			__fluent_runtime::LocalizationManifest::parse(CATALOG_CONFIG, CATALOG_ASSET_PATH)
				.expect("build-validated localization manifest")
		}
	}
}

pub(super) fn implementation(scopes: &[Scope]) -> Item {
	let modules: Vec<Expr> = scopes
		.iter()
		.filter_map(|scope| {
			let path = scope.module_path.as_deref()?;
			let ty = &scope.type_path;
			Some(
				parse_quote!(__fluent_runtime::Module::new::<#ty>(#path, |locale, bytes| {
					#ty::new(locale, bytes).map_err(|error| error.to_string())
				})),
			)
		})
		.collect();
	let types = scopes.iter().map(|scope| &scope.type_path);

	parse_quote! {
		impl __fluent_runtime::FluentCatalog for Translations {
			type Locale = Locale;
			type Modules<'a> = __bevy_views::Scope0<'a>;

			fn locales() -> &'static [Locale] { Locale::iter().as_slice() }

			fn default_locale() -> Locale { DEFAULT_LANGUAGE.parse().expect("build-validated default locale") }

			fn source_locale() -> Locale { SOURCE_LANGUAGE.parse().expect("build-validated source locale") }

			fn modules() -> ::std::vec::Vec<__fluent_runtime::Module<Self>> { ::std::vec![#(#modules,)*] }

			fn scopes() -> ::std::vec::Vec<__fluent_runtime::ScopeRegistration<Self>> {
				::std::vec![#(__fluent_runtime::ScopeRegistration::new::<#types>(),)*]
			}

			fn view(modules: &__fluent_runtime::ModuleStore<Self>) -> Self::Modules<'_> {
				__bevy_views::Scope0 { modules }
			}
		}
	}
}
