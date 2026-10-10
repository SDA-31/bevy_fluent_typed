#[cfg(feature = "manifest")]
use crate::LocalizationManifest;
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{
	FluentCatalog, FluentScope, Lazy, Localization, LocalizedText, Message, Module, ModuleStore,
	ScopeRegistration, bindings,
};
use std::sync::Arc;

mod automatic;
#[cfg(feature = "manifest")]
mod bindings_lifecycle;
mod bytes;
mod documentation;
#[cfg(feature = "manifest")]
mod lazy;
#[cfg(feature = "manifest")]
mod loader;
mod preparation;
#[cfg(feature = "manifest")]
mod scheduling;
mod spans;
mod switching;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
mod template;
#[cfg(feature = "manifest")]
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

#[cfg(feature = "manifest")]
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
fn selected_locale_preserves_active_data_and_logical_requests_until_ready() {
	let mut state = Localization::<TestCatalog, Lazy>::default();
	assert_eq!(state.locale(), "ja");
	assert!(state.catalog().is_none());
	state.load::<TestCatalog>();
	state
		.store
		.insert_leaf("ui.ftl", Arc::new(TestCatalog("ready".into())));
	assert_eq!(state.catalog().unwrap().0, "ready");
	state.set_locale("es");
	assert_eq!(state.catalog().unwrap().0, "ready");
	assert_eq!(state.locale(), "ja");
	assert_eq!(state.prepared_locale(), Some("es"));
	assert!(state.desired().contains("ui.ftl"));
	state.unload::<TestCatalog>();
	assert!(state.desired().is_empty());
}

#[test]
fn deferred_messages_refresh_and_clear_existing_ui_and_world_labels() {
	let message = Message::<TestCatalog>::new(|catalog| catalog.0.clone());
	let cloned = message.clone();
	let binding = LocalizedText::from(message);
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
		.spawn((Text::default(), binding.clone()))
		.id();
	let world = app.world_mut().spawn((Text2d::default(), binding)).id();
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

#[test]
fn default_ui_respects_explicit_targets_and_waits_for_a_catalog() {
	let mut app = App::new();
	bindings::register::<TestCatalog>(&mut app, false);
	let binding = LocalizedText::<TestCatalog>::new(|catalog| catalog.0.clone());
	let implicit = app.world_mut().spawn(binding.clone()).id();
	let ui = app
		.world_mut()
		.spawn((binding.clone(), Text::default()))
		.id();
	let world = app
		.world_mut()
		.spawn((binding.clone(), Text2d::default()))
		.id();
	let late_world = app.world_mut().spawn(binding.clone()).id();
	app.world_mut()
		.entity_mut(late_world)
		.insert(Text2d::default());
	let cancelled = app.world_mut().spawn(binding.clone()).id();
	app.world_mut()
		.entity_mut(cancelled)
		.remove::<LocalizedText<TestCatalog>>();
	let despawned = app.world_mut().spawn(binding).id();
	app.world_mut().despawn(despawned);
	app.update();

	assert!(app.world().get::<Text>(implicit).unwrap().0.is_empty());
	assert!(app.world().get::<Text>(ui).unwrap().0.is_empty());
	assert!(app.world().get::<Text>(world).is_none());
	assert!(app.world().get::<Text>(late_world).is_none());
	assert!(app.world().get::<Text>(cancelled).is_none());
	app.world_mut().insert_resource(TestCatalog("ready".into()));
	app.update();

	for entity in [implicit, ui] {
		assert_eq!(app.world().get::<Text>(entity).unwrap().0, "ready");
	}

	for entity in [world, late_world] {
		assert_eq!(app.world().get::<Text2d>(entity).unwrap().0, "ready");
		assert!(app.world().get::<Text>(entity).is_none());
	}
}
