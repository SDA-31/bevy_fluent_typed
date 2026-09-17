use crate::bevy::prelude::*;
use crate::{
	CatalogDescriptor, FluentCatalog, Localization, LocalizedText, Message, Module, ModuleSource,
	bindings,
};
use std::sync::Arc;

mod documentation;
mod loader;
mod scheduling;

// Deliberately no Clone, Default or TypePath: generated catalogs need none.
struct TestCatalog(String);

impl FluentCatalog for TestCatalog {
	type Locale = &'static str;

	fn locales() -> &'static [Self::Locale] {
		&["de", "es", "ja"]
	}

	fn default_locale() -> Self::Locale {
		"ja"
	}

	fn embedded(locale: Self::Locale) -> Self {
		Self(locale.into())
	}

	fn descriptor(_: &[u8]) -> Result<CatalogDescriptor, String> {
		Ok(CatalogDescriptor {
			modules_directory: ".".into(),
		})
	}

	fn modules(_: Self::Locale) -> Vec<Module> {
		vec![Module {
			path: "ui.ftl",
			embedded: "hello = Hi\n",
		}]
	}

	fn parse(_: Self::Locale, sources: &[ModuleSource<'_>]) -> Result<Self, String> {
		Ok(Self(sources.iter().map(|module| module.source).collect()))
	}
}

#[test]
fn arbitrary_locales_default_and_inactive_reload_keep_their_own_catalogs() {
	let mut state = Localization::<TestCatalog>::default();
	assert_eq!(state.locale(), "ja");
	state.publish(
		"es",
		Arc::new(TestCatalog("changed".into())),
		&[("ui.ftl", "changed".into())],
	);
	assert_eq!(state.catalog().0, "ja");
	state.set_locale("es");
	assert_eq!(state.catalog().0, "changed");
	state.set_locale("de");
	assert_eq!(state.catalog().0, "de");
}

#[test]
fn deferred_messages_and_existing_ui_world_labels_follow_the_active_catalog() {
	let message = Message::<TestCatalog>::new(|catalog| catalog.0.clone());
	let cloned = message.clone();
	let mut app = App::new();
	app.init_resource::<Localization<TestCatalog>>()
		.add_systems(
			Update,
			(
				bindings::refresh_ui::<TestCatalog>,
				bindings::refresh_world::<TestCatalog>,
			),
		);
	let ui = app
		.world_mut()
		.spawn((Text::default(), LocalizedText::from(message)))
		.id();
	let world = app
		.world_mut()
		.spawn((Text2d::default(), LocalizedText::from(cloned.clone())))
		.id();
	app.update();
	assert_eq!(app.world().get::<Text>(ui).unwrap().0, "ja");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	app.update();
	assert_eq!(app.world().get::<Text>(ui).unwrap().0, "es");
	assert_eq!(app.world().get::<Text2d>(world).unwrap().0, "es");
	assert_eq!(
		cloned.render(
			app.world()
				.resource::<Localization<TestCatalog>>()
				.catalog()
		),
		"es"
	);
}
