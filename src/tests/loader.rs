//! Prove the runtime accepts opaque providers and preserves named asset sources.
use crate::bevy::{
	asset::io::{
		AssetSourceBuilder,
		memory::{Dir, MemoryAssetReader},
	},
	ecs as bevy_ecs,
	prelude::*,
};
use crate::{
	CatalogConfig, CatalogUpdate, CatalogUpdateReader, FluentCatalog, FluentScope, Localization,
	LocalizationManifest, LocalizationPlugin, LocalizationSystems, Module, ModuleStore,
	ReloadCatalogs, ScopeRegistration,
};
use std::{
	fs,
	path::{Path, PathBuf},
	sync::atomic::{AtomicU64, Ordering},
	time::{Duration, Instant},
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const DEFINITION: &[u8] = b"\xffopaque-localization-v1";
const ASSET_PATH: &str = "test://nested/catalog.definition";

#[derive(Resource, Clone)]
struct OpaqueProvider(String);

impl FluentScope for OpaqueProvider {
	type Catalog = Self;
	fn module_paths() -> &'static [&'static str] {
		&["ui/title.ftl"]
	}

	fn assemble(modules: &ModuleStore<Self>) -> Option<Self> {
		modules.get::<Self>().ok().cloned()
	}
}

impl FluentCatalog for OpaqueProvider {
	type Locale = &'static str;
	type Modules<'a> = &'a ModuleStore<Self>;

	fn locales() -> &'static [Self::Locale] {
		&["en", "fr"]
	}

	fn default_locale() -> Self::Locale {
		"en"
	}

	fn modules() -> Vec<Module<Self>> {
		vec![Module::new::<Self>("ui/title.ftl", |_, bytes| {
			let source = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
			let value = source.strip_prefix("title = ").ok_or("missing title")?;
			Ok(Self(value.trim().into()))
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
	LocalizationManifest::from_config(
		CatalogConfig {
			source_language: "en".into(),
			default_language: "en".into(),
			languages_directory: "translations".into(),
		},
		ASSET_PATH,
	)
}

struct Fixture(PathBuf);

impl Fixture {
	fn new() -> Self {
		let id = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
		let directory =
			std::env::temp_dir().join(format!("bevy-fluent-provider-{}-{id}", std::process::id()));
		fs::create_dir(&directory).unwrap();
		fs::create_dir_all(directory.join("nested/translations/en/ui")).unwrap();
		Self(directory)
	}

	fn write_definition(&self, bytes: &[u8]) {
		fs::write(self.0.join("nested/catalog.definition"), bytes).unwrap();
	}

	fn write_title(&self, title: &str) {
		fs::write(
			self.0.join("nested/translations/en/ui/title.ftl"),
			format!("title = {title}\n"),
		)
		.unwrap();
	}
}

impl Drop for Fixture {
	fn drop(&mut self) {
		// Only the uniquely created fixture directory belongs to this test.
		let _ = fs::remove_dir_all(&self.0);
	}
}

#[derive(Resource, Default)]
struct Failures(usize);

fn observe(mut events: CatalogUpdateReader<OpaqueProvider>, mut failures: ResMut<Failures>) {
	for event in events.read() {
		if matches!(event, CatalogUpdate::Rejected { .. }) {
			failures.0 += 1;
		}
	}
}

fn pump_until(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if ready(app.world()) {
			return;
		}

		assert!(Instant::now() < deadline, "asset loading timed out");
		std::thread::sleep(Duration::from_millis(5));
	}
}

fn request_reload(world: &mut World) {
	#[cfg(feature = "bevy-0-16")]
	world.send_event(ReloadCatalogs::<OpaqueProvider>::default());

	#[cfg(not(feature = "bevy-0-16"))]
	world.write_message(ReloadCatalogs::<OpaqueProvider>::default());
}

#[test]
fn explicit_contract_named_source_and_last_good_recovery_work_without_codegen() {
	let fixture = Fixture::new();
	fixture.write_definition(DEFINITION);
	fixture.write_title("External");
	let mut app = App::new();
	app.register_asset_source(
		"test",
		AssetSourceBuilder::platform_default(fixture.0.to_str().unwrap(), None),
	)
	.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			watch_for_changes_override: Some(false),
			..default()
		},
	))
	.init_resource::<Failures>()
	.add_plugins(LocalizationPlugin::<OpaqueProvider>::new(manifest()))
	.add_systems(PreUpdate, observe.after(LocalizationSystems::Publish));
	app.finish();
	app.cleanup();
	pump_until(&mut app, |world| {
		world
			.resource::<Localization<OpaqueProvider>>()
			.catalog()
			.is_some_and(|catalog| catalog.0 == "External")
	});

	fs::write(
		fixture.0.join("nested/translations/en/ui/title.ftl"),
		b"invalid module",
	)
	.unwrap();
	request_reload(app.world_mut());
	pump_until(&mut app, |world| world.resource::<Failures>().0 > 0);
	assert_eq!(
		app.world()
			.resource::<Localization<OpaqueProvider>>()
			.catalog()
			.unwrap()
			.0,
		"External"
	);

	fixture.write_title("Not yet published");
	request_reload(app.world_mut());
	pump_until(&mut app, |world| {
		world
			.resource::<Localization<OpaqueProvider>>()
			.catalog()
			.is_some_and(|catalog| catalog.0 == "Not yet published")
	});
}

fn memory_source(root: Dir) -> AssetSourceBuilder {
	let reader = move || {
		Box::new(MemoryAssetReader { root: root.clone() })
			as Box<dyn crate::bevy::asset::io::ErasedAssetReader>
	};

	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19"))]
	{
		AssetSourceBuilder::new(reader)
	}

	#[cfg(not(any(feature = "bevy-0-18", feature = "bevy-0-19")))]
	{
		AssetSourceBuilder::default().with_reader(reader)
	}
}

#[test]
fn virtual_source_without_files_or_watcher_reloads_and_recovers_from_missing_modules() {
	let files = Dir::default();
	files.insert_asset(Path::new("nested/catalog.definition"), DEFINITION);
	files.insert_asset_text(
		Path::new("nested/translations/en/ui/title.ftl"),
		"title = Virtual\n",
	);
	let mut app = App::new();
	app.register_asset_source("test", memory_source(files.clone()))
		.add_plugins((
			MinimalPlugins,
			AssetPlugin {
				watch_for_changes_override: Some(false),
				..default()
			},
		))
		.init_resource::<Failures>()
		.add_plugins(LocalizationPlugin::<OpaqueProvider>::new(manifest()))
		.add_systems(PreUpdate, observe.after(LocalizationSystems::Publish));
	app.finish();
	app.cleanup();
	pump_until(&mut app, |world| {
		world
			.resource::<Localization<OpaqueProvider>>()
			.catalog()
			.is_some_and(|catalog| catalog.0 == "Virtual")
	});
	files.remove_asset(Path::new("nested/translations/en/ui/title.ftl"));
	request_reload(app.world_mut());
	pump_until(&mut app, |world| world.resource::<Failures>().0 == 1);
	assert_eq!(
		app.world()
			.resource::<Localization<OpaqueProvider>>()
			.catalog()
			.unwrap()
			.0,
		"Virtual"
	);
	files.insert_asset_text(
		Path::new("nested/translations/en/ui/title.ftl"),
		"title = Recovered\n",
	);
	request_reload(app.world_mut());
	pump_until(&mut app, |world| {
		world
			.resource::<Localization<OpaqueProvider>>()
			.catalog()
			.is_some_and(|catalog| catalog.0 == "Recovered")
	});
}

#[test]
fn named_source_address_index_distinguishes_locales_for_the_same_module() {
	let files = Dir::default();
	files.insert_asset_text(
		Path::new("nested/translations/en/ui/title.ftl"),
		"title = English\n",
	);
	files.insert_asset_text(
		Path::new("nested/translations/fr/ui/title.ftl"),
		"title = French\n",
	);
	let mut app = App::new();
	app.register_asset_source("test", memory_source(files))
		.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.add_plugins(LocalizationPlugin::<OpaqueProvider>::new(manifest()));
	app.finish();
	app.cleanup();

	for (locale, expected) in [("en", "English"), ("fr", "French"), ("en", "English")] {
		app.world_mut()
			.resource_mut::<Localization<OpaqueProvider>>()
			.set_locale(locale);
		pump_until(&mut app, |world| {
			world
				.resource::<Localization<OpaqueProvider>>()
				.catalog()
				.is_some_and(|catalog| catalog.0 == expected)
		});
	}

	// Let asset notifications settle. Tracking handles must not mark the
	// controller changed on older backends when no new candidate exists.
	for _ in 0..4 {
		app.update();
	}

	let changed = app
		.world()
		.get_resource_ref::<Localization<OpaqueProvider>>()
		.unwrap()
		.last_changed();

	for _ in 0..4 {
		app.update();
	}

	assert_eq!(
		app.world()
			.get_resource_ref::<Localization<OpaqueProvider>>()
			.unwrap()
			.last_changed(),
		changed
	);
}
