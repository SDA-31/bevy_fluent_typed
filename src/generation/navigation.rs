//! Views borrow the store rather than temporary intermediate navigation values.
use fluent_typed_codegen::{
	Scope,
	syn::{Ident, ImplItemFn, Item, parse_quote},
};

pub(super) fn implementation(scopes: &[Scope]) -> Item {
	let mut items: Vec<Item> = Vec::new();

	for (index, scope) in scopes
		.iter()
		.enumerate()
		.filter(|(_, scope)| scope.module_path.is_none())
	{
		let name: Ident = fluent_typed_codegen::syn::parse_str(&format!("Scope{index}")).unwrap();
		let methods: Vec<ImplItemFn> = scopes.iter().enumerate().filter(|(_, child)| {
			child.accessors.len() == scope.accessors.len() + 1 && child.accessors.starts_with(&scope.accessors)
		}).map(|(child_index, child)| {
			let accessor = child.accessors.last().unwrap();

			if child.module_path.is_some() {
				let ty = &child.type_path;
				parse_quote! {
					/// Borrow this leaf if ready for the selected locale; never starts I/O.
					pub fn #accessor(self) -> ::std::result::Result<&'a super::#ty, super::__fluent_runtime::ModuleError> {
						self.modules.get::<super::#ty>()
					}
				}
			} else {
				let child_name: Ident = fluent_typed_codegen::syn::parse_str(&format!("Scope{child_index}")).unwrap();
				parse_quote! {
					/// Navigate the schema even when this group's data is incomplete.
					pub fn #accessor(self) -> #child_name<'a> { #child_name { modules: self.modules } }
				}
			}
		}).collect();
		items.push(parse_quote! {
			/// Borrowed schema navigation; data remains owned by the controller.
			#[derive(Clone, Copy)]
			pub struct #name<'a> {
				pub(super) modules: &'a super::__fluent_runtime::ModuleStore<super::Translations>,
			}
		});
		items.push(parse_quote! { impl<'a> #name<'a> { #(#methods)* } });
	}

	parse_quote! {
		#[doc(hidden)]
		pub mod __bevy_views { #(#items)* }
	}
}
