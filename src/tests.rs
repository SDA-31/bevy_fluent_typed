use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{
	FluentCatalog, FluentScope, Lazy, Localization, LocalizationManifest, LocalizedText, Message,
	Module, ModuleStore, ScopeRegistration, bindings,
};
use std::sync::Arc;

mod bindings_lifecycle;
mod documentation;
mod lazy;
mod loader;
mod scheduling;
mod waiting;

#[derive(Resource, Clone)]
struct TestCatalog(String);

impl FluentScope for TestCatalog {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&["ui.ftl"]
	}

	fn assemble(modules: &ModuleStore<Self>) -> Option<Self> {
		modules.get::<Self>().ok().cloned()
	}
}

impl FluentCatalog for TestCatalog {
	type Locale = &'static str;
	type Modules<'a> = &'a ModuleStore<Self>;

	fn locales() -> &'static [Self::Locale] {
		&["de", "es", "ja"]
	}

	fn default_locale() -> Self::Locale {
		"ja"
	}

	fn modules() -> Vec<Module<Self>> {
		vec![Module::new::<Self>("ui.ftl", |_, bytes| {
			String::from_utf8(bytes.to_vec())
				.map(Self)
				.map_err(|error| error.to_string())
		})]
	}

	fn scopes() -> Vec<ScopeRegistration<Self>> {
		vec![ScopeRegistration::new::<Self>()]
	}

	fn view(modules: &ModuleStore<Self>) -> Self::Modules<'_> {
		modules
	}
}

fn manifest() -> LocalizationManifest {
	LocalizationManifest::__embedded((
		"ja",
		"ja",
		".",
		&[
			("de", "ui.ftl", b"de"),
			("es", "ui.ftl", b"es"),
			("ja", "ui.ftl", b"ja"),
		],
	))
}

#[test]
fn selected_locale_releases_previous_data_but_keeps_logical_requests() {
	let mut state = Localization::<TestCatalog, Lazy>::default();
	assert_eq!(state.locale(), "ja");
	assert!(state.catalog().is_none());
	state.load::<TestCatalog>();
	state
		.store
		.insert_leaf("ui.ftl", Arc::new(TestCatalog("ready".into())));
	assert_eq!(state.catalog().unwrap().0, "ready");
	state.set_locale("es");
	assert!(state.catalog().is_none());
	assert!(state.desired().contains("ui.ftl"));
	state.unload::<TestCatalog>();
	assert!(state.desired().is_empty());
}

#[test]
fn deferred_messages_refresh_and_clear_existing_ui_and_world_labels() {
	let message = Message::<TestCatalog>::new(|catalog| catalog.0.clone());
	let cloned = message.clone();
	let mut app = App::new();
	app.insert_resource(TestCatalog("ja".into())).add_systems(
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
	app.world_mut().insert_resource(TestCatalog("es".into()));
	app.update();
	assert_eq!(app.world().get::<Text>(ui).unwrap().0, "es");
	assert_eq!(app.world().get::<Text2d>(world).unwrap().0, "es");
	assert_eq!(cloned.render(app.world().resource()), "es");
	app.world_mut().remove_resource::<TestCatalog>();
	app.update();
	assert!(app.world().get::<Text>(ui).unwrap().0.is_empty());
	assert!(app.world().get::<Text2d>(world).unwrap().0.is_empty());
}
