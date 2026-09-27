//! Root provider descriptors carry schema and parsers, never embedded source text.
use fluent_typed_codegen::{
	Scope,
	syn::{Expr, Item, parse_quote},
};

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
