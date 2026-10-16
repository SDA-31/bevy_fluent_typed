use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Member, Path, parse_macro_input, parse_quote};

/// Bind one String field through the public LocalizationTarget contract.
#[proc_macro_derive(LocalizationTarget, attributes(localized, localization))]
pub fn localization_target(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);

	match expand(input) {
		Ok(output) => quote!(#output).into(),
		Err(error) => error.to_compile_error().into(),
	}
}

fn expand(mut input: DeriveInput) -> syn::Result<impl quote::ToTokens> {
	let mut runtime: Path = parse_quote!(::bevy_fluent_typed);

	for attribute in &input.attrs {
		if !attribute.path().is_ident("localization") {
			continue;
		}

		attribute.parse_nested_meta(|meta| {
			if !meta.path.is_ident("crate") {
				return Err(meta.error("expected `crate = runtime_crate_path`"));
			}

			runtime = meta.value()?.parse()?;
			Ok(())
		})?;
	}

	let syn::Data::Struct(data) = &input.data else {
		return Err(syn::Error::new_spanned(
			&input.ident,
			"LocalizationTarget requires a struct",
		));
	};
	let mut selected = None;

	for (index, field) in data.fields.iter().enumerate() {
		if !field
			.attrs
			.iter()
			.any(|attribute| attribute.path().is_ident("localized"))
		{
			continue;
		}

		for attribute in &field.attrs {
			if attribute.path().is_ident("localized")
				&& !matches!(attribute.meta, syn::Meta::Path(_))
			{
				return Err(syn::Error::new_spanned(
					attribute,
					"use #[localized] without arguments",
				));
			}
		}

		if selected.is_some() {
			return Err(syn::Error::new_spanned(
				field,
				"select exactly one #[localized] String field",
			));
		}

		selected = Some(
			field
				.ident
				.clone()
				.map_or_else(|| Member::Unnamed(index.into()), Member::Named),
		);
	}

	let field = selected.ok_or_else(|| {
		syn::Error::new_spanned(
			&input.ident,
			"mark one String field with #[localized]; use a manual adapter for multiple outputs",
		)
	})?;
	let name = &input.ident;
	let (_, arguments, _) = input.generics.split_for_impl();
	let component = parse_quote!(#name #arguments: #runtime::bevy::prelude::Component<
		Mutability = #runtime::bevy::ecs::component::Mutable
	> + ::core::default::Default);
	input
		.generics
		.make_where_clause()
		.predicates
		.push(component);
	let (generics, arguments, predicates) = input.generics.split_for_impl();

	Ok(quote! {
		impl #generics #runtime::LocalizationTarget for #name #arguments #predicates {
			type Data = #runtime::bevy::prelude::Mut<'static, Self>;
			type Filter = ();

			fn initialize(entity: &mut #runtime::bevy::prelude::EntityWorldMut<'_>) {
				if !entity.contains::<Self>() {
					entity.insert(<Self as ::core::default::Default>::default());
				}
			}

			#[inline]
			fn needs_update(
				data: &#runtime::TargetItem<'_, '_, Self>,
				update: &#runtime::LocalizationUpdate<'_>,
			) -> bool {
				update.is_changed() || #runtime::bevy::prelude::DetectChanges::is_changed(data)
			}

			#[inline]
			fn update(
				target: #runtime::TargetItem<'_, '_, Self>,
				update: &mut #runtime::LocalizationUpdate<'_>,
			) -> #runtime::UpdateResult {
				update.set_text(target.map_unchanged(|target| &mut target.#field));

				#runtime::UpdateResult::Done
			}

			fn apply(
				entity: &mut #runtime::bevy::prelude::EntityWorldMut<'_>,
				update: &mut #runtime::LocalizationUpdate<'_>,
			) {
				let Some(target) = entity.get_mut::<Self>() else {
					return;
				};
				update.set_text(target.map_unchanged(|target| &mut target.#field));
			}

			fn matches(target: #runtime::TargetReadItem<'_, '_, Self>, text: &str) -> bool {
				target.#field == text
			}
		}
	})
}

#[cfg(test)]
mod tests {
	use super::expand;
	use syn::parse_quote;

	#[test]
	fn malformed_selection_is_rejected_with_specific_diagnostics() {
		for (input, expected) in [
			(
				parse_quote!(
					enum Caption {
						Empty,
					}
				),
				"requires a struct",
			),
			(
				parse_quote!(
					struct Caption(String);
				),
				"mark one String field",
			),
			(
				parse_quote!(
					struct Caption(#[localized] String, #[localized] String);
				),
				"exactly one",
			),
			(
				parse_quote!(
					struct Caption(#[localized(skip)] String);
				),
				"without arguments",
			),
			(
				parse_quote!(
					#[localization(other = x)]
					struct Caption(#[localized] String);
				),
				"runtime_crate_path",
			),
		] {
			let error = match expand(input) {
				Ok(_) => panic!("invalid derive accepted"),
				Err(error) => error,
			};
			assert!(error.to_string().contains(expected), "{error}");
		}
	}
}
