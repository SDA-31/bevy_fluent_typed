//! Ownership regression: the acquired allocation must reach the parsed snapshot.
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::tests::{TestCatalog, pump};
use crate::{
	FluentCatalog, FluentScope, Localization, LocalizationPlugin, Manual, Module, ModuleStore,
	ScopeRegistration,
};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Clone)]
struct OwnedCatalog(Arc<String>);

impl FluentScope for OwnedCatalog {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&["ui.ftl"]
	}

	fn assemble(modules: &ModuleStore<Self>) -> Option<Self> {
		modules.get::<Self>().ok().cloned()
	}
}

impl FluentCatalog for OwnedCatalog {
	type Locale = &'static str;
	type Modules<'a> = &'a ModuleStore<Self>;

	fn locales() -> &'static [Self::Locale] {
		&["ja", "es"]
	}

	fn default_locale() -> Self::Locale {
		"ja"
	}

	fn modules() -> Vec<Module<Self>> {
		vec![Module::new::<Self>("ui.ftl", |_, bytes| {
			String::from_utf8(bytes)
				.map(|source| Self(Arc::new(source)))
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

#[test]
fn owned_loader_transfers_the_original_allocation_through_locale_changes() {
	let acquired = Arc::new(AtomicUsize::new(0));
	let pointer = acquired.clone();
	let plugin = LocalizationPlugin::<OwnedCatalog, Manual>::from_loader(move |locale, _| {
		let bytes = locale.as_bytes().to_vec();
		pointer.store(bytes.as_ptr() as usize, Ordering::Relaxed);
		std::future::ready(Ok::<_, String>(bytes))
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();
	app.world_mut()
		.resource_mut::<Localization<OwnedCatalog, Manual>>()
		.load::<OwnedCatalog>();

	for locale in ["ja", "es", "ja"] {
		app.world_mut()
			.resource_mut::<Localization<OwnedCatalog, Manual>>()
			.set_locale(locale);
		pump(&mut app, |world| {
			world
				.get_resource::<OwnedCatalog>()
				.is_some_and(|value| value.0.as_str() == locale)
		});

		let parsed = app.world().resource::<OwnedCatalog>();
		assert_eq!(parsed.0.as_ptr() as usize, acquired.load(Ordering::Relaxed));
	}
}

#[test]
fn string_loader_results_use_the_standard_parser_contract() {
	let plugin = LocalizationPlugin::<TestCatalog, Manual>::from_loader(|_, _| {
		std::future::ready(Ok::<_, String>(String::from("string input")))
	});
	let mut app = super::app(plugin);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());

	assert_eq!(app.world().resource::<TestCatalog>().0, "string input");
}
