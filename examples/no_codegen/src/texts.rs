//! Handwritten checked provider for one argument-free Fluent leaf/root.
use bevy_fluent_typed::bevy::{ecs as bevy_ecs, prelude::Resource};
use bevy_fluent_typed::fluent_typed::prelude::L10nBundle;
use bevy_fluent_typed::{FluentCatalog, FluentScope, Module, ModuleStore, ScopeRegistration};

#[derive(Resource, Clone)]
pub(super) struct Texts {
	pub(super) hello: String,
}

impl FluentScope for Texts {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&["ui/greeting.ftl"]
	}

	fn assemble(modules: &ModuleStore<Self>) -> Option<Self> {
		modules.get::<Self>().ok().cloned()
	}
}

impl FluentCatalog for Texts {
	type Locale = &'static str;
	type Modules<'a> = &'a ModuleStore<Self>;

	fn locales() -> &'static [Self::Locale] {
		&["en", "es", "ru"]
	}

	fn default_locale() -> Self::Locale {
		"en"
	}

	fn modules() -> Vec<Module<Self>> {
		vec![Module::new::<Self>("ui/greeting.ftl", Self::parse)]
	}

	fn scopes() -> Vec<ScopeRegistration<Self>> {
		vec![ScopeRegistration::new::<Self>()]
	}

	fn view(modules: &ModuleStore<Self>) -> Self::Modules<'_> {
		modules
	}
}

impl Texts {
	pub(super) fn parse(locale: &str, bytes: &[u8]) -> Result<Self, String> {
		let bundle = L10nBundle::new(locale, bytes).map_err(|error| error.to_string())?;
		let hello = bundle
			.msg("hello", None)
			.map_err(|error| error.to_string())?;
		Ok(Self { hello })
	}
}
