//! The engine-free tree can also be included directly at a consumer crate root.
#![allow(dead_code, clippy::derivable_impls, clippy::too_many_arguments)]
// The generated validator accesses this dependency through its parent scope.
#[allow(clippy::single_component_path_imports)]
use fluent_syntax;
use localization_runtime::fluent_typed;

include!(concat!(env!("OUT_DIR"), "/translations.rs"));

#[test]
fn raw_root_include_and_standard_library_names_do_not_collide() {
	let catalog: Translations = Locale::En.load();
	let presentation: &Presentation = catalog.presentation();
	let hud: &presentation::Hud = presentation.hud();
	assert_eq!(hud.msg_title(), "Flight HUD");
	assert_eq!(catalog.vec().msg_title(), "Example: vec");
	assert_eq!(catalog.as_ref().msg_title(), "Example: as-ref");
	assert_eq!(catalog.ok().msg_title(), "Example: ok");
	assert_eq!(catalog.err().msg_title(), "Example: err");
	assert_eq!(catalog.string().msg_title(), "Example: string");
	assert_eq!(catalog.r#type().msg_title(), "Example: type");
	assert!(Translations::from_modules(Locale::En, &[]).is_err());
}

#[test]
fn translations_root_leaves_catalog_available_as_a_domain_module() {
	let translations: Translations = Locale::En.load();
	let catalog: &Catalog = translations.catalog();
	assert_eq!(catalog.msg_title(), "Product catalog");
}
