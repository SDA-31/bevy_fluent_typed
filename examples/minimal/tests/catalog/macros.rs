use localization_runtime::LocalizationManifest;

mod fixture {
	use localization_runtime::translations;

	// Caller scope must not override the macro's defining runtime dependency.
	mod bevy_fluent_typed {}

	translations!(
		/// A nested module with restricted visibility and an optional semicolon.
		pub(super) mod texts;
	);

	// Forwarded attributes must apply to the whole generated module.
	translations!(
		#[cfg(any())]
		mod texts
	);
}

#[test]
fn macro_resolves_the_renamed_runtime_and_consumer_build_output() {
	let translated: fixture::texts::Translations = fixture::texts::Translations::from_manifest(
		fixture::texts::Locale::En,
		&fixture::texts::embed_manifest!(),
	)
	.unwrap();
	let hud: &fixture::texts::presentation::Hud = translated.presentation().hud();
	let expected = super::load(crate::texts::Locale::En);

	assert_eq!(
		translated.ui().msg_example_greeting("Ada"),
		expected.ui().msg_example_greeting("Ada")
	);
	assert_eq!(hud.prompt().s0, expected.presentation().hud().prompt().s0);
	assert_eq!(
		LocalizationManifest::parse(fixture::texts::CATALOG_CONFIG, "catalog.toml")
			.unwrap()
			.config()
			.languages_directory,
		LocalizationManifest::parse(crate::texts::CATALOG_CONFIG, "catalog.toml")
			.unwrap()
			.config()
			.languages_directory
	);
	assert_eq!(fixture::texts::MODULES, crate::texts::MODULES);
}

#[test]
fn generated_manifest_uses_build_configuration_without_loading_sources() {
	let manifest = fixture::texts::manifest();

	assert_eq!(
		manifest.file_path(),
		Some(std::path::Path::new(fixture::texts::CATALOG_ASSET_PATH))
	);
	assert_eq!(
		manifest.config().source_language,
		fixture::texts::SOURCE_LANGUAGE
	);
	assert_eq!(
		manifest.config().default_language,
		fixture::texts::DEFAULT_LANGUAGE
	);
	assert_eq!(
		manifest.config().languages_directory,
		std::path::Path::new(fixture::texts::LANGUAGES_DIRECTORY)
	);
	assert!(manifest.embedded_modules().is_none());
}

#[test]
fn embedded_constants_select_scopes_and_preserve_catalog_aliases() {
	use fixture::texts::presentation::Hud as FlightHud;

	fixture::texts::embed_manifest! {
		const SELECTED = presentation::Hud;
		const GROUP = Presentation;
		const COMPLETE = Translations;
	}

	let selected: LocalizationManifest = SELECTED;
	let direct = fixture::texts::embed_manifest!(module = fixture::texts::presentation::Hud);
	assert_eq!(selected.embedded_modules(), direct.embedded_modules());
	assert!(
		selected
			.embedded_modules()
			.unwrap()
			.iter()
			.all(|(_, path, _)| *path == "presentation/hud.ftl")
	);
	let hud = FlightHud::from_manifest(fixture::texts::Locale::En, &selected).unwrap();
	assert_eq!(hud.msg_title(), "Flight HUD");

	let group: LocalizationManifest = GROUP;
	assert_eq!(
		group.embedded_modules(),
		fixture::texts::embed_manifest!(module = fixture::texts::Presentation).embedded_modules()
	);
	assert_eq!(
		COMPLETE.embedded_modules(),
		fixture::texts::embed_manifest!().embedded_modules()
	);
	assert!(
		group
			.embedded_modules()
			.unwrap()
			.iter()
			.all(|(_, path, _)| path.starts_with("presentation/"))
	);
	let group_hud = FlightHud::from_manifest(fixture::texts::Locale::En, &group).unwrap();
	fixture::texts::presentation::Panel::from_manifest(fixture::texts::Locale::En, &group).unwrap();
	assert_eq!(group_hud.msg_title(), hud.msg_title());
	assert!(group.read("en", "ui.ftl").is_err());
}

// The generated tree stays private; consumers only import the chosen application API.
mod encapsulated {
	localization_runtime::translations!(mod catalogs);

	pub use catalogs::presentation::Hud as Interface;
	pub use catalogs::{Locale, Translations};

	catalogs::embed_manifest! {
		pub const HUD = presentation::Hud;
	}
}

#[test]
fn exported_constant_and_catalog_alias_support_lazy_required_resources() {
	use encapsulated::{HUD, Interface, Locale, Translations};
	use localization_runtime::bevy::prelude::*;
	use localization_runtime::{Lazy, Localization, LocalizationAppExt, LocalizationPlugin};
	use std::sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	};

	let invocations = Arc::new(AtomicUsize::new(0));
	let observed = invocations.clone();
	let manifest: LocalizationManifest = HUD;
	assert!(
		manifest
			.embedded_modules()
			.unwrap()
			.iter()
			.all(|(_, path, _)| *path == "presentation/hud.ftl")
	);
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<Translations, Lazy>::new(HUD),
	))
	.add_localized_systems(Update, move |_: Res<Interface>| {
		observed.fetch_add(1, Ordering::Relaxed);
	});
	app.finish();
	app.cleanup();
	app.update();
	assert_eq!(invocations.load(Ordering::Relaxed), 0);
	assert!(!app.world().contains_resource::<Interface>());

	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.load::<Interface>();
	app.update();
	assert_eq!(invocations.load(Ordering::Relaxed), 1);
	assert_eq!(
		app.world().resource::<Interface>().msg_title(),
		"Flight HUD"
	);
	assert!(!app.world().contains_resource::<Translations>());

	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.set_locale(Locale::Es);
	app.update();
	assert_eq!(invocations.load(Ordering::Relaxed), 2);
	assert_eq!(
		app.world().resource::<Interface>().msg_title(),
		"Panel de vuelo"
	);

	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.unload::<Interface>();
	app.update();
	assert_eq!(invocations.load(Ordering::Relaxed), 2);
	assert!(!app.world().contains_resource::<Interface>());

	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.load::<Interface>();
	app.update();
	assert_eq!(invocations.load(Ordering::Relaxed), 3);
	assert_eq!(
		app.world().resource::<Interface>().msg_title(),
		"Panel de vuelo"
	);
}
