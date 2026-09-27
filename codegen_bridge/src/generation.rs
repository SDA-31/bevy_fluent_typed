//! The only source emitter that knows both generator and Bevy runtime APIs.
use crate::{navigation, provider, scopes};
use fluent_typed_codegen::{
	Extension, Scope, Settings,
	syn::{Item, ItemStruct, ItemUse, parse_quote},
};
use std::{path::Path, process::ExitCode};

struct BevyExtension;

impl Extension for BevyExtension {
	fn filename(&self) -> &str {
		"bevy_catalog.rs"
	}

	fn root_imports(&self) -> Vec<ItemUse> {
		vec![
			parse_quote!(
				use __fluent_runtime::__fluent_codegen;
			),
			parse_quote!(
				use __fluent_bridge::fluent_syntax;
			),
			parse_quote!(
				use __fluent_runtime::fluent_typed;
			),
			parse_quote! {
				#[allow(unused_imports)]
				use __fluent_runtime::bevy::ecs as bevy_ecs;
			},
		]
	}

	fn scope_imports(&self) -> Vec<ItemUse> {
		vec![
			parse_quote!(
				use super::__fluent_runtime;
			),
			parse_quote! {
				#[allow(unused_imports)]
				use __fluent_runtime::bevy::ecs as bevy_ecs;
			},
		]
	}

	fn type_declaration(&self, declaration: ItemStruct) -> Item {
		parse_quote! {
			__fluent_runtime::__localized_resource! { #declaration }
		}
	}

	fn reserved_names(&self) -> &[&str] {
		&["bevy_ecs"]
	}

	fn root_items(&self, metadata: &[Scope]) -> Vec<Item> {
		let mut items = scopes::implementations(metadata);
		items.push(provider::implementation(metadata));
		items.push(navigation::implementation(metadata));
		items
	}
}

/// Build-script entrypoint: discover catalogs and emit their Bevy adapter.
/// On generation failure, prints diagnostics to stderr and returns a failure
/// ExitCode without terminating the process. Return it from build.rs main so Cargo
/// observes generation failures.
#[must_use = "return this exit code from build.rs main"]
pub fn build() -> ExitCode {
	fluent_typed_codegen::build_with(&BevyExtension)
}

/// Generate from the consuming package's Cargo metadata and OUT_DIR.
/// Reads `CARGO_MANIFEST_DIR` and `OUT_DIR`, writes the plain and Bevy entrypoints,
/// and emits Cargo rerun directives to stdout.
///
/// # Errors
/// Reports missing Cargo environment, invalid catalogs/settings, generated-name
/// or syntax errors, and filesystem failures.
pub fn from_cargo() -> Result<(), String> {
	fluent_typed_codegen::from_cargo_with(&BevyExtension)
}

/// Generate with explicit package/output paths, without Cargo environment lookup.
/// Settings paths are relative to `package`. Creates `output` and overwrites owned
/// outputs, including `bevy_catalog.rs`, leaving unrelated files untouched.
/// Writes persistent staging files and Cargo rerun directives through the generator.
/// Keep output in a tool-owned target directory; generation is not transactional,
/// so discard its results after a failure.
///
/// # Errors
/// Reports the same validation and I/O failures as [`from_cargo`].
pub fn generate(package: &Path, output: &Path, settings: &Settings) -> Result<(), String> {
	fluent_typed_codegen::generate_with(package, output, settings, &BevyExtension)
}
