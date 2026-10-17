#[cfg(feature = "manifest")]
use crate::LocalizationManifest;
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{
	FluentCatalog, FluentScope, Localization, Localized, Manual, Module, ModuleStore, Msg,
	ScopeRegistration, bindings,
};
use std::sync::Arc;

mod automatic;
#[cfg(feature = "manifest")]
mod bindings_lifecycle;
mod bytes;
mod custom_bindings;
mod documentation;
#[cfg(feature = "manifest")]
mod lazy;
mod leases;
#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17", feature = "bevy-0-18"))]
mod legacy_systems;
#[cfg(feature = "manifest")]
mod loader;
mod msg;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
mod native_resources;
mod preparation;
mod progress;
mod progress_registration;
mod registration;
#[cfg(feature = "manifest")]
mod scheduling;
mod spans;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
mod startup;
mod switching;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
mod template;
#[cfg(all(
	feature = "manifest",
	any(feature = "bevy-0-19", feature = "bevy-0-20")
))]
mod waiting;

#[derive(Resource, Clone)]
struct TestCatalog(String);

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
type OptionalCatalog<'w> = Option<crate::PassiveCatalog<'w, TestCatalog>>;

#[cfg(not(any(feature = "bevy-0-19", feature = "bevy-0-20")))]
type OptionalCatalog<'w> = Option<Res<'w, TestCatalog>>;

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
	let mut state = Localization::<TestCatalog, Manual>::default();
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
	let message = Msg::<TestCatalog>::new(|catalog| catalog.0.clone());
	let cloned = message.clone();
	let binding = Localized::<Text, _>::from(message.clone());
	let mut app = App::new();
	bindings::register::<TestCatalog>(&mut app, false);
	app.insert_resource(TestCatalog("ja".into()));
	let ui = app
		.world_mut()
		.spawn((Text::default(), binding.clone()))
		.id();
	let world = app
		.world_mut()
		.spawn((Text2d::default(), Localized::<Text2d, _>::from(message)))
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

#[test]
fn native_defaults_create_only_the_selected_target_without_a_catalog() {
	let mut app = App::new();
	bindings::register::<TestCatalog>(&mut app, false);
	let binding = Localized::<Text, TestCatalog>::new(|catalog| catalog.0.clone());
	let implicit = app.world_mut().spawn(binding.clone()).id();
	let ui = app
		.world_mut()
		.spawn((binding.clone(), Text::default()))
		.id();
	let world = app
		.world_mut()
		.spawn((
			Localized::<Text2d, TestCatalog>::new(|catalog| catalog.0.clone()),
			Text2d::default(),
		))
		.id();
	let late_world = app
		.world_mut()
		.spawn(Localized::<Text2d, TestCatalog>::new(|catalog| {
			catalog.0.clone()
		}))
		.id();
	app.world_mut()
		.entity_mut(late_world)
		.insert(Text2d::default());
	let cancelled = app.world_mut().spawn(binding.clone()).id();
	app.world_mut()
		.entity_mut(cancelled)
		.remove::<Localized<Text, TestCatalog>>();
	let despawned = app.world_mut().spawn(binding).id();
	app.world_mut().despawn(despawned);
	app.update();

	assert!(app.world().get::<Text>(implicit).unwrap().0.is_empty());
	assert!(app.world().get::<Text>(ui).unwrap().0.is_empty());
	assert!(app.world().get::<Text>(world).is_none());
	assert!(app.world().get::<Text>(late_world).is_none());
	// Initialization may already have run when an exclusive world access flushed hooks.
	assert!(app.world().get::<Text>(cancelled).unwrap().0.is_empty());
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
