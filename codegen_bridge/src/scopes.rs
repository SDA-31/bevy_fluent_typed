//! Scope contracts assemble parents from already parsed child snapshots.
use fluent_typed_codegen::{
	Scope,
	syn::{Item, parse_quote},
};

pub(super) fn implementations(scopes: &[Scope]) -> Vec<Item> {
	scopes.iter().map(|scope| {
		let ty = &scope.type_path;
		let paths: Vec<_> = scopes.iter().filter(|candidate| {
			candidate.accessors.starts_with(&scope.accessors)
		}).filter_map(|candidate| candidate.module_path.as_deref()).collect();
		let assembly: fluent_typed_codegen::syn::Expr = if scope.module_path.is_some() {
			parse_quote!(modules.get::<Self>().ok().cloned())
		} else {
			let children = scopes.iter().filter(|candidate| candidate.accessors.len() == scope.accessors.len() + 1
				&& candidate.accessors.starts_with(&scope.accessors)).map(|child| &child.type_path);
			parse_quote!(Self::__from_parts(modules.locale(), #(modules.get::<#children>().ok()?.clone(),)*).ok())
		};

		parse_quote! {
			impl __fluent_runtime::FluentScope for #ty {
				type Catalog = Translations;

				fn module_paths() -> &'static [&'static str] { &[#(#paths,)*] }

				fn assemble(modules: &__fluent_runtime::ModuleStore<Translations>) -> ::std::option::Option<Self> {
					#assembly
				}
			}
		}
	}).collect()
}
