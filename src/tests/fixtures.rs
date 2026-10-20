#[cfg(feature = "manifest")]
use crate::LocalizationManifest;
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{FluentCatalog, FluentScope, Module, ModuleStore, ScopeRegistration};
use std::time::{Duration, Instant};

#[derive(Resource, Clone)]
pub(crate) struct TestCatalog(pub(crate) String);

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
pub(crate) type OptionalCatalog<'w> = Option<crate::PassiveCatalog<'w, TestCatalog>>;

#[cfg(not(any(feature = "bevy-0-19", feature = "bevy-0-20")))]
pub(crate) type OptionalCatalog<'w> = Option<Res<'w, TestCatalog>>;

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
pub(crate) fn manifest() -> LocalizationManifest {
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

pub(crate) fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if ready(app.world()) {
			return;
		}

		assert!(Instant::now() < deadline, "byte source timed out");
		std::thread::sleep(Duration::from_millis(1));
	}
}
