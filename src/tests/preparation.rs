//! Transactional locale changes keep the active resource graph usable.
use super::TestCatalog;
#[cfg(feature = "manifest")]
use crate::bevy::ecs as bevy_ecs;
use crate::bevy::prelude::*;
use crate::{
	CommitLocaleError, Full, Lazy, LoadingMode, Localization, LocalizationAppExt,
	LocalizationPlugin, LocalizedText, PreparationStatus,
};
use std::{
	sync::{
		Arc, Mutex,
		atomic::{AtomicUsize, Ordering},
	},
	time::{Duration, Instant},
};

fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if ready(app.world()) {
			return;
		}

		assert!(Instant::now() < deadline, "locale preparation timed out");
		std::thread::sleep(Duration::from_millis(1));
	}
}

fn bytes_app<M: LoadingMode>() -> App {
	let plugin = LocalizationPlugin::<TestCatalog, M>::from_bytes([
		("ja", "ui.ftl", b"Japanese".as_slice()),
		("es", "ui.ftl", b"Spanish".as_slice()),
		("de", "ui.ftl", &[0xff]),
	])
	.unwrap();
	let mut state = Localization::<TestCatalog, M>::default();
	// Both policies begin with the same requested leaf for this fixture.
	state
		.requested
		.insert(std::any::TypeId::of::<TestCatalog>(), &["ui.ftl"]);
	state.desired = Arc::new(["ui.ftl"].into_iter().collect());
	let mut app = App::new();
	app.insert_resource(state)
		.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app
}

fn exercise_bytes<M: LoadingMode>() {
	let mut app = bytes_app::<M>();
	let label = app
		.world_mut()
		.spawn((
			Text::default(),
			LocalizedText::<TestCatalog>::new(|catalog| catalog.0.clone()),
		))
		.id();
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	app.add_localized_systems(Update, move |catalog: Res<TestCatalog>| {
		assert!(!catalog.0.is_empty());
		observed.fetch_add(1, Ordering::Relaxed);
	});
	app.update();
	let initial_revision = app
		.world()
		.resource::<Localization<TestCatalog, M>>()
		.store
		.revision;
	{
		let mut localization = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, M>>();
		assert_eq!(
			localization.commit_locale(),
			Err(CommitLocaleError::NotPrepared)
		);
		localization.prepare_locale("ja");
		assert_eq!(localization.preparation_status(), PreparationStatus::Ready);
		localization.prepare_locale("ja");
		localization.commit_locale().unwrap();
	}
	app.update();
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, M>>()
			.store
			.revision,
		initial_revision
	);
	let before = calls.load(Ordering::Relaxed);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, M>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, M>>()
			.preparation_status()
			== PreparationStatus::Ready
	});
	assert!(calls.load(Ordering::Relaxed) > before);
	assert_eq!(app.world().resource::<TestCatalog>().0, "Japanese");
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "Japanese");
	{
		let mut localization = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, M>>();
		assert_eq!(localization.prepared_locale(), Some("es"));
		localization.commit_locale().unwrap();
		assert_eq!(localization.locale(), "ja");
		assert_eq!(localization.catalog().unwrap().0, "Japanese");
	}
	app.update();
	let state = app.world().resource::<Localization<TestCatalog, M>>();
	assert_eq!(state.locale(), "es");
	assert_eq!(state.preparation_status(), PreparationStatus::Idle);
	assert!(state.store.revision > initial_revision);
	assert_eq!(state.catalog().unwrap().0, "Spanish");
	assert_eq!(app.world().resource::<TestCatalog>().0, "Spanish");
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "Spanish");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, M>>()
		.prepare_locale("de");
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog, M>>()
				.preparation_status(),
			PreparationStatus::Failed(_)
		)
	});
	assert!(matches!(
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, M>>()
			.commit_locale(),
		Err(CommitLocaleError::Failed(_))
	));
	assert_eq!(app.world().resource::<TestCatalog>().0, "Spanish");
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "Spanish");
}

#[test]
fn bytes_full_and_lazy_publish_only_on_explicit_commit() {
	exercise_bytes::<Full>();
	exercise_bytes::<Lazy>();
}

#[test]
fn failed_target_retry_does_not_accept_a_retained_old_good_snapshot() {
	let payload = Arc::new(Mutex::new(Ok::<_, String>(b"first".to_vec())));
	let source = payload.clone();
	let plugin = LocalizationPlugin::<TestCatalog, Lazy>::from_loader(move |_, _| {
		std::future::ready(source.lock().unwrap().clone())
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation_status()
			== PreparationStatus::Ready
	});
	*payload.lock().unwrap() = Err("offline".into());
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(crate::ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog, Lazy>>()
				.preparation_status(),
			PreparationStatus::Failed(_)
		)
	});
	assert!(matches!(
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.commit_locale(),
		Err(CommitLocaleError::Failed(_))
	));
	assert_eq!(app.world().resource::<TestCatalog>().0, "first");
	*payload.lock().unwrap() = Ok(b"repaired".to_vec());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation_status(),
		PreparationStatus::Preparing
	);
	assert_eq!(
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.commit_locale(),
		Err(CommitLocaleError::Pending)
	);
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation_status()
			== PreparationStatus::Ready
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.locale(),
		"es"
	);
	assert_eq!(app.world().resource::<TestCatalog>().0, "repaired");
}

#[test]
fn empty_lazy_demand_and_immediate_locale_selection_supersede_preparation() {
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let plugin = LocalizationPlugin::<TestCatalog, Lazy>::from_loader(move |locale, _| {
		observed.fetch_add(1, Ordering::Relaxed);
		std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	{
		let mut localization = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>();
		localization.prepare_locale("es");
		assert_eq!(localization.preparation_status(), PreparationStatus::Ready);
		localization.commit_locale().unwrap();
	}
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), 0);
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.locale(),
		"es"
	);
	{
		let mut localization = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>();
		localization.prepare_locale("de");
		localization.set_locale("es");
		assert_eq!(localization.preparation_status(), PreparationStatus::Idle);
		localization.prepare_locale("de");
		localization.load::<TestCatalog>();
		assert_eq!(
			localization.commit_locale(),
			Err(CommitLocaleError::Pending)
		);
		localization.unload::<TestCatalog>();
		assert_eq!(localization.preparation_status(), PreparationStatus::Ready);
		localization.cancel_preparation();
	}
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[cfg(feature = "manifest")]
#[test]
fn embedded_manifest_preparation_publishes_no_target_resources_before_commit() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<TestCatalog>::new(super::manifest()),
	));
	app.update();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("es");
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.preparation_status(),
		PreparationStatus::Ready
	);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");
}

#[cfg(feature = "manifest")]
#[test]
fn filesystem_target_preparation_uses_normal_asset_loader_and_active_reload() {
	use crate::{CatalogConfig, LocalizationManifest};
	use std::{fs, sync::atomic::AtomicU64};

	static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);
	struct Fixture(std::path::PathBuf);

	impl Fixture {
		fn replace(&self, relative: &str, bytes: impl AsRef<[u8]>) {
			let destination = self.0.join(relative);
			let temporary = destination.with_extension("pending");
			fs::write(&temporary, bytes).unwrap();
			fs::rename(temporary, destination).unwrap();
		}
	}

	impl Drop for Fixture {
		fn drop(&mut self) {
			let _ = fs::remove_dir_all(&self.0);
		}
	}

	let fixture = Fixture(std::env::temp_dir().join(format!(
		"fluent-preparation-{}-{}",
		std::process::id(),
		NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
	)));
	for locale in ["ja", "es"] {
		fs::create_dir_all(fixture.0.join("data").join(locale)).unwrap();
		fs::write(fixture.0.join("data").join(locale).join("ui.ftl"), locale).unwrap();
	}
	let manifest = LocalizationManifest::from_config(
		CatalogConfig {
			source_language: "ja".into(),
			default_language: "ja".into(),
			languages_directory: "data".into(),
		},
		"catalog.toml",
	);
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: fixture.0.to_string_lossy().into_owned(),
			watch_for_changes_override: Some(false),
			..default()
		},
		LocalizationPlugin::<TestCatalog>::new(manifest),
	));
	app.finish();
	app.cleanup();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog>>()
			.preparation_status()
			== PreparationStatus::Ready
	});
	let state = app.world().resource::<Localization<TestCatalog>>();
	let entry = &state.preparation.as_ref().unwrap().entries["ui.ftl"];
	assert!(entry.preparation_request.is_some());
	assert!(
		entry
			.preparation_handle
			.as_ref()
			.unwrap()
			.path()
			.unwrap()
			.label()
			.is_none()
	);
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");
	assert!(
		app.world().resource::<Localization<TestCatalog>>().entries["ui.ftl"]
			.handle
			.as_ref()
			.unwrap()
			.path()
			.unwrap()
			.label()
			.is_none()
	);
	// Commit reconnects the normal asset namespace asynchronously. Wait for
	// that read to publish before exercising a separate explicit reload.
	let normal_read_settled = |world: &World| {
		let state = world.resource::<Localization<TestCatalog>>();
		let entry = &state.entries["ui.ftl"];
		let asset = entry.handle.as_ref().and_then(|handle| {
			world
				.resource::<Assets<crate::assets::ModuleAsset<TestCatalog>>>()
				.get(handle)
		});

		!entry.pending
			&& state.retry.is_empty()
			&& asset.is_some_and(|asset| entry.accepted == Some(asset.revision))
	};
	pump(&mut app, normal_read_settled);

	// A source update must never expose the truncated intermediate file to
	// an asynchronous reader; the fixture replaces complete byte snapshots.
	fixture.replace("data/es/ui.ftl", "updated");
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(crate::ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
	pump(&mut app, |world| {
		world.resource::<TestCatalog>().0 == "updated" && normal_read_settled(world)
	});
	fixture.replace("data/es/ui.ftl", [0xff]);
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(crate::ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog>>()
				.status::<TestCatalog>(),
			crate::ModuleStatus::Failed(_)
		)
	});
	assert_eq!(app.world().resource::<TestCatalog>().0, "updated");
}

#[cfg(feature = "manifest")]
#[derive(Resource, Clone)]
struct SecondCatalog(String);

#[cfg(feature = "manifest")]
impl crate::FluentScope for SecondCatalog {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&["ui.ftl"]
	}

	fn assemble(modules: &crate::ModuleStore<Self>) -> Option<Self> {
		modules.get::<Self>().ok().cloned()
	}
}

#[cfg(feature = "manifest")]
impl crate::FluentCatalog for SecondCatalog {
	type Locale = &'static str;
	type Modules<'a> = &'a crate::ModuleStore<Self>;

	fn locales() -> &'static [Self::Locale] {
		&["ja", "es"]
	}

	fn default_locale() -> Self::Locale {
		"ja"
	}

	fn modules() -> Vec<crate::Module<Self>> {
		vec![crate::Module::new::<Self>("ui.ftl", |_, bytes| {
			Ok(Self(format!(
				"second:{}",
				std::str::from_utf8(bytes).map_err(|error| error.to_string())?
			)))
		})]
	}

	fn scopes() -> Vec<crate::ScopeRegistration<Self>> {
		vec![crate::ScopeRegistration::new::<Self>()]
	}

	fn view(modules: &crate::ModuleStore<Self>) -> Self::Modules<'_> {
		modules
	}
}

#[cfg(feature = "manifest")]
#[test]
fn preparation_preserves_foreign_ftl_loader_and_multiple_provider_namespaces() {
	use crate::bevy::{
		asset::{
			self as bevy_asset, AssetLoader, LoadContext,
			io::{
				AssetSourceBuilder, Reader,
				memory::{Dir, MemoryAssetReader},
			},
		},
		tasks::futures_lite::future,
	};
	use crate::{CatalogConfig, LocalizationManifest};
	use std::{any::type_name, path::Path};

	#[derive(Asset)]
	struct ForeignAsset;

	impl TypePath for ForeignAsset {
		fn type_path() -> &'static str {
			type_name::<Self>()
		}

		fn short_type_path() -> &'static str {
			type_name::<Self>()
		}
	}

	struct ForeignLoader;

	impl TypePath for ForeignLoader {
		fn type_path() -> &'static str {
			type_name::<Self>()
		}

		fn short_type_path() -> &'static str {
			type_name::<Self>()
		}
	}

	impl AssetLoader for ForeignLoader {
		type Asset = ForeignAsset;
		type Settings = ();
		type Error = std::io::Error;

		fn extensions(&self) -> &[&str] {
			&["ftl"]
		}

		async fn load(
			&self,
			reader: &mut dyn Reader,
			_: &(),
			_: &mut LoadContext<'_>,
		) -> Result<Self::Asset, Self::Error> {
			let mut bytes = Vec::new();
			reader.read_to_end(&mut bytes).await?;
			Ok(ForeignAsset)
		}
	}

	let files = Dir::default();
	for locale in ["ja", "es"] {
		files.insert_asset_text(&Path::new("data").join(locale).join("ui.ftl"), locale);
	}
	let manifest = LocalizationManifest::from_config(
		CatalogConfig {
			source_language: "ja".into(),
			default_language: "ja".into(),
			languages_directory: "data".into(),
		},
		"independent://contract.toml",
	);
	let reader = move || {
		Box::new(MemoryAssetReader {
			root: files.clone(),
		}) as Box<dyn crate::bevy::asset::io::ErasedAssetReader>
	};
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20"))]
	let source = AssetSourceBuilder::new(reader);
	#[cfg(not(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20")))]
	let source = AssetSourceBuilder::default().with_reader(reader);
	let mut app = App::new();
	app.register_asset_source("independent", source)
		.add_plugins((MinimalPlugins, AssetPlugin::default()));
	app.init_asset::<ForeignAsset>()
		.register_asset_loader(ForeignLoader);
	app.add_plugins((
		LocalizationPlugin::<TestCatalog>::new(manifest.clone()),
		LocalizationPlugin::<SecondCatalog>::new(manifest),
	));
	app.finish();
	app.cleanup();
	let server = app.world().resource::<AssetServer>();
	let foreign = server.load::<ForeignAsset>("independent://data/es/ui.ftl");
	let registered = future::block_on(server.get_asset_loader_with_extension("ftl")).unwrap();
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20"))]
	assert_eq!(registered.type_path(), ForeignLoader::type_path());
	#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17"))]
	assert_eq!(registered.type_name(), ForeignLoader::type_path());
	pump(&mut app, |world| {
		world.contains_resource::<TestCatalog>()
			&& world.contains_resource::<SecondCatalog>()
			&& world.resource::<Assets<ForeignAsset>>().contains(&foreign)
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("es");
	app.world_mut()
		.resource_mut::<Localization<SecondCatalog>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog>>()
			.preparation_status()
			== PreparationStatus::Ready
			&& world
				.resource::<Localization<SecondCatalog>>()
				.preparation_status()
				== PreparationStatus::Ready
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.commit_locale()
		.unwrap();
	app.world_mut()
		.resource_mut::<Localization<SecondCatalog>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");
	assert_eq!(app.world().resource::<SecondCatalog>().0, "second:es");
	let server = app.world().resource::<AssetServer>();
	let registered = future::block_on(server.get_asset_loader_with_extension("ftl")).unwrap();
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20"))]
	assert_eq!(registered.type_path(), ForeignLoader::type_path());
	#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17"))]
	assert_eq!(registered.type_name(), ForeignLoader::type_path());
}

#[cfg(feature = "manifest")]
#[test]
fn matching_settings_on_another_catalog_loader_fail_without_entering_expected_loader() {
	use crate::bevy::asset::io::{
		AssetSourceBuilder,
		memory::{Dir, MemoryAssetReader},
	};
	use crate::{CatalogConfig, LocalizationManifest};
	use std::path::Path;

	let files = Dir::default();
	files.insert_asset_text(Path::new("data/ja/ui.ftl"), "ja");
	files.insert_asset_text(Path::new("data/es/ui.ftl"), "es");
	let loader = std::any::type_name::<crate::assets::PreparedModuleLoader<SecondCatalog>>();
	files.insert_meta_text(
		Path::new("data/es/ui.ftl"),
		&format!(
			"(meta_format_version: \"1.0\", asset: Load(loader: \"{loader}\", settings: (0, ())))"
		),
	);
	let manifest = LocalizationManifest::from_config(
		CatalogConfig {
			source_language: "ja".into(),
			default_language: "ja".into(),
			languages_directory: "data".into(),
		},
		"wrong-provider://contract.toml",
	);
	let reader = move || {
		Box::new(MemoryAssetReader {
			root: files.clone(),
		}) as Box<dyn crate::bevy::asset::io::ErasedAssetReader>
	};
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20"))]
	let source = AssetSourceBuilder::new(reader);
	#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17"))]
	let source = AssetSourceBuilder::default().with_reader(reader);
	let mut app = App::new();
	app.register_asset_source("wrong-provider", source)
		.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			LocalizationPlugin::<TestCatalog>::new(manifest.clone()),
			LocalizationPlugin::<SecondCatalog>::new(manifest),
		));
	app.finish();
	app.cleanup();
	pump(&mut app, |world| {
		world.contains_resource::<TestCatalog>() && world.contains_resource::<SecondCatalog>()
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog>>()
				.preparation_status(),
			PreparationStatus::Failed(_)
		)
	});
	let state = app.world().resource::<Localization<TestCatalog>>();
	let attempt = state.preparation.as_ref().unwrap().entries["ui.ftl"]
		.preparation_attempt
		.as_ref()
		.unwrap();
	assert!(attempt.settings_applied.load(Ordering::Acquire));
	assert!(attempt.finished_without_loader());
	assert!(matches!(
		state.preparation_status(),
		PreparationStatus::Failed(_)
	));
	assert_eq!(state.locale(), "ja");
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	assert_eq!(app.world().resource::<SecondCatalog>().0, "second:ja");
	assert_eq!(
		app.world()
			.resource::<crate::loading::CatalogSource<TestCatalog>>()
			.attempts
			.len(),
		1
	);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.cancel_preparation();
	assert_eq!(
		app.world()
			.resource::<crate::loading::CatalogSource<TestCatalog>>()
			.attempts
			.len(),
		0
	);
}
