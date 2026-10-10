use super::{TestCatalog, manifest};
use crate::bevy::asset::io::{
	AssetReader, AssetReaderError, AssetSourceBuilder, PathStream, Reader,
	memory::{Dir, MemoryAssetReader},
};
use crate::bevy::prelude::*;
use crate::bevy::tasks::futures_lite::io::AsyncRead;
use crate::{
	Full, Lazy, Localization, LocalizationPlugin, LocalizationProgressPlugin, LocalizedText,
	ModuleStatus,
};
use std::{
	any::TypeId,
	path::{Path, PathBuf},
	pin::Pin,
	sync::{
		Arc, Mutex,
		atomic::{AtomicBool, Ordering},
	},
	task::{Context, Poll, Waker},
	time::{Duration, Instant},
};

#[derive(Default)]
struct Gate {
	state: Mutex<(bool, Vec<PathBuf>, Vec<Waker>)>,
	in_loader: AtomicBool,
	pass_new_reads: AtomicBool,
}

impl Gate {
	fn poll(&self, cx: &Context<'_>) -> Poll<()> {
		let mut state = self.state.lock().unwrap();

		if state.0 {
			Poll::Ready(())
		} else {
			state.2.push(cx.waker().clone());
			Poll::Pending
		}
	}

	fn release(&self) {
		let mut state = self.state.lock().unwrap();
		state.0 = true;

		for waker in state.2.drain(..) {
			waker.wake();
		}
	}
}

struct ControlledStream<R> {
	inner: R,
	gate: Arc<Gate>,
	gated: bool,
}

impl<R: Reader> AsyncRead for ControlledStream<R> {
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		bytes: &mut [u8],
	) -> Poll<std::io::Result<usize>> {
		if self.gated && self.gate.poll(cx).is_pending() {
			return Poll::Pending;
		}

		Pin::new(&mut self.inner).poll_read(cx, bytes)
	}
}

#[cfg(not(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20")))]
impl<R: Reader> crate::bevy::asset::io::AsyncSeekForward for ControlledStream<R> {
	fn poll_seek_forward(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
		offset: u64,
	) -> Poll<std::io::Result<u64>> {
		Pin::new(&mut self.inner).poll_seek_forward(cx, offset)
	}
}

impl<R: Reader> Reader for ControlledStream<R> {
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20"))]
	fn seekable(
		&mut self,
	) -> Result<
		&mut dyn crate::bevy::asset::io::SeekableReader,
		crate::bevy::asset::io::ReaderNotSeekableError,
	> {
		self.inner.seekable()
	}
}

struct ControlledReader {
	inner: MemoryAssetReader,
	gate: Arc<Gate>,
}

impl AssetReader for ControlledReader {
	async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
		let gated = !self.gate.pass_new_reads.load(Ordering::SeqCst);
		let in_loader = self.gate.in_loader.load(Ordering::SeqCst);
		self.gate.state.lock().unwrap().1.push(path.to_owned());
		let reader = self.inner.read(path).await;

		if gated && !in_loader {
			std::future::poll_fn(|cx| self.gate.poll(cx)).await;
		}

		Ok(ControlledStream {
			inner: reader?,
			gate: self.gate.clone(),
			gated: gated && in_loader,
		})
	}

	async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
		self.inner.read_meta(path).await
	}

	async fn read_directory<'a>(
		&'a self,
		path: &'a Path,
	) -> Result<Box<PathStream>, AssetReaderError> {
		self.inner.read_directory(path).await
	}

	async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
		self.inner.is_directory(path).await
	}
}

fn asynchronous_configured() -> (App, Arc<Gate>, Dir) {
	let files = Dir::default();
	for locale in ["ja", "es", "de"] {
		files.insert_asset_text(
			&Path::new("nested/data").join(locale).join("ui.ftl"),
			locale,
		);
	}
	let gate = Arc::new(Gate::default());
	let root = files.clone();
	let reader_gate = gate.clone();
	let reader = move || {
		Box::new(ControlledReader {
			inner: MemoryAssetReader { root: root.clone() },
			gate: reader_gate.clone(),
		}) as Box<dyn crate::bevy::asset::io::ErasedAssetReader>
	};
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20"))]
	let source = AssetSourceBuilder::new(reader);
	#[cfg(not(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20")))]
	let source = AssetSourceBuilder::default().with_reader(reader);
	let contract = crate::LocalizationManifest::from_config(
		crate::CatalogConfig {
			source_language: "ja".into(),
			default_language: "ja".into(),
			languages_directory: "data".into(),
		},
		"controlled://nested/manifest.toml",
	);
	let mut app = App::new();
	app.register_asset_source("controlled", source)
		.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			LocalizationPlugin::<TestCatalog>::new_lazy(contract),
		));
	(app, gate, files)
}

fn asynchronous_app() -> (App, Arc<Gate>, Dir) {
	let (mut app, gate, files) = asynchronous_configured();
	app.finish();
	app.cleanup();
	(app, gate, files)
}

fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);
	loop {
		app.update();
		if ready(app.world()) {
			return;
		}
		assert!(
			Instant::now() < deadline,
			"controlled asset request timed out"
		);
		std::thread::sleep(Duration::from_millis(5));
	}
}

fn request_reload(world: &mut World) {
	#[cfg(feature = "bevy-0-16")]
	world.send_event(crate::ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	world.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
}

#[test]
fn our_reload_requests_serialize_before_loader_entry_and_coalesce_without_losing_retry() {
	for failure in ["none", "schema", "io"] {
		let (mut app, gate, files) = asynchronous_app();

		if failure == "schema" {
			files.insert_asset(Path::new("nested/data/ja/ui.ftl"), vec![0xff]);
		} else if failure == "io" {
			files.remove_asset(Path::new("nested/data/ja/ui.ftl"));
		}

		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.load::<TestCatalog>();
		pump(&mut app, |_| !gate.state.lock().unwrap().2.is_empty());
		files.insert_asset_text(Path::new("nested/data/ja/ui.ftl"), "fresh");
		request_reload(app.world_mut());
		app.update();
		request_reload(app.world_mut());
		app.update();
		assert_eq!(
			gate.state.lock().unwrap().1.len(),
			1,
			"no second read before initial completion"
		);
		gate.release();
		pump(&mut app, |world| {
			world
				.get_resource::<TestCatalog>()
				.is_some_and(|catalog| catalog.0 == "fresh")
		});
		assert_eq!(
			gate.state.lock().unwrap().1.len(),
			2,
			"pending retries coalesce into one fresh read"
		);
		assert!(
			app.world()
				.resource::<Localization<TestCatalog, Lazy>>()
				.retry
				.is_empty()
		);
	}
}

#[test]
fn older_loader_finishing_after_newer_loader_cannot_regress_published_value() {
	let (mut app, gate, files) = asynchronous_app();
	gate.in_loader.store(true, Ordering::SeqCst);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| !gate.state.lock().unwrap().2.is_empty());
	files.insert_asset_text(Path::new("nested/data/ja/ui.ftl"), "newer");
	gate.pass_new_reads.store(true, Ordering::SeqCst);
	// Deliberately bypass the serialized public localization API to exercise
	// out-of-order Reader::read_to_end completion inside Bevy's loaders.
	app.world()
		.resource::<AssetServer>()
		.reload("controlled://nested/data/ja/ui.ftl");
	pump(&mut app, |world| {
		world
			.get_resource::<TestCatalog>()
			.is_some_and(|catalog| catalog.0 == "newer")
	});
	let accepted = app
		.world()
		.resource::<Localization<TestCatalog, Lazy>>()
		.entries["ui.ftl"]
		.accepted
		.unwrap();
	gate.release();
	pump(&mut app, |world| {
		let state = world.resource::<Localization<TestCatalog, Lazy>>();
		let handle = state.entries["ui.ftl"].handle.as_ref().unwrap();
		world
			.resource::<Assets<crate::assets::ModuleAsset<TestCatalog>>>()
			.get(handle)
			.is_some_and(|asset| asset.revision < accepted)
	});
	assert_eq!(app.world().resource::<TestCatalog>().0, "newer");
}

#[test]
fn no_io_until_demand_and_pending_locale_results_cannot_publish_into_current_language() {
	let (mut app, gate, _) = asynchronous_app();
	for _ in 0..3 {
		app.update();
	}
	assert!(gate.state.lock().unwrap().1.is_empty());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| !gate.state.lock().unwrap().1.is_empty());
	assert!(!app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.set_locale("es");
	pump(&mut app, |_| gate.state.lock().unwrap().1.len() == 2);
	gate.release();
	pump(&mut app, |world| {
		world
			.get_resource::<TestCatalog>()
			.is_some_and(|catalog| catalog.0 == "es")
	});
	for _ in 0..3 {
		app.update();
		assert_eq!(app.world().resource::<TestCatalog>().0, "es");
	}
	let paths = &gate.state.lock().unwrap().1;
	assert_eq!(
		paths,
		&[
			PathBuf::from("nested/data/ja/ui.ftl"),
			PathBuf::from("nested/data/es/ui.ftl"),
		]
	);
}

#[test]
fn cancelled_pending_load_releases_owned_assets_and_explicit_retry_recovers() {
	let (mut app, gate, files) = asynchronous_app();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| !gate.state.lock().unwrap().1.is_empty());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.unload::<TestCatalog>();
	app.update();
	gate.release();
	pump(&mut app, |world| {
		world
			.resource::<Assets<crate::assets::ModuleAsset<TestCatalog>>>()
			.is_empty()
	});
	assert!(!app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	let weak = Arc::downgrade(
		&app.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.store
			.values[&TypeId::of::<TestCatalog>()]
			.value,
	);
	let clone = app.world().resource::<TestCatalog>().clone();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.unload::<TestCatalog>();
	pump(&mut app, |_| weak.upgrade().is_none());
	assert_eq!(clone.0, "ja");
	files.remove_asset(Path::new("nested/data/ja/ui.ftl"));
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog, Lazy>>()
				.status::<TestCatalog>(),
			ModuleStatus::Failed(_)
		)
	});
	files.insert_asset_text(Path::new("nested/data/ja/ui.ftl"), "repaired");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world
			.get_resource::<TestCatalog>()
			.is_some_and(|catalog| catalog.0 == "repaired")
	});
}

#[test]
fn required_system_waits_through_io_failure_and_retry_without_blocking_optional_system() {
	use crate::LocalizationAppExt;
	use std::sync::atomic::AtomicUsize;

	let (mut app, gate, files) = asynchronous_app();
	let required_runs = Arc::new(AtomicUsize::new(0));
	let optional_runs = Arc::new(AtomicUsize::new(0));
	let required_counter = required_runs.clone();
	let optional_counter = optional_runs.clone();
	app.add_localized_systems(
		Update,
		(
			move |_: Res<TestCatalog>| {
				required_counter.fetch_add(1, Ordering::Relaxed);
			},
			move |_: Option<Res<TestCatalog>>| {
				optional_counter.fetch_add(1, Ordering::Relaxed);
			},
		),
	);
	files.remove_asset(Path::new("nested/data/ja/ui.ftl"));
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| !gate.state.lock().unwrap().1.is_empty());
	assert_eq!(required_runs.load(Ordering::Relaxed), 0);
	assert!(optional_runs.load(Ordering::Relaxed) > 0);
	gate.release();
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog, Lazy>>()
				.status::<TestCatalog>(),
			ModuleStatus::Failed(_)
		)
	});
	assert_eq!(required_runs.load(Ordering::Relaxed), 0);
	files.insert_asset_text(Path::new("nested/data/ja/ui.ftl"), "repaired");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| required_runs.load(Ordering::Relaxed) > 0);
	let before = required_runs.load(Ordering::Relaxed);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.unload::<TestCatalog>();
	app.update();
	assert_eq!(required_runs.load(Ordering::Relaxed), before);
}

#[test]
fn malformed_source_contract_is_rejected_without_panicking() {
	for (source, default, origin) in [
		("ja", "unknown", "manifest.toml"),
		("ja", "ja", "bad#source://manifest.toml"),
	] {
		let contract = crate::LocalizationManifest::from_config(
			crate::CatalogConfig {
				source_language: source.into(),
				default_language: default.into(),
				languages_directory: "data".into(),
			},
			origin,
		);
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			LocalizationPlugin::<TestCatalog>::new(contract),
		));
		app.finish();
		app.cleanup();
		app.update();
		assert!(!app.world().contains_resource::<TestCatalog>());
		assert!(matches!(
			app.world()
				.resource::<Localization<TestCatalog>>()
				.status::<TestCatalog>(),
			ModuleStatus::Failed(_)
		));
	}
}

#[test]
fn lazy_constructors_wait_for_explicit_demand_and_clear_bindings_on_unload() {
	for short in [false, true] {
		let plugin: LocalizationPlugin<TestCatalog, Lazy> = if short {
			LocalizationPlugin::<TestCatalog>::new_lazy(manifest())
		} else {
			LocalizationPlugin::<TestCatalog, Lazy>::new(manifest())
		};
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default(), plugin));
		let label = app
			.world_mut()
			.spawn((
				Text("old".into()),
				LocalizedText::new(|catalog: &TestCatalog| catalog.0.clone()),
			))
			.id();
		app.finish();
		app.cleanup();
		app.update();
		assert!(!app.world().contains_resource::<TestCatalog>());
		assert!(app.world().get::<Text>(label).unwrap().0.is_empty());
		assert_eq!(
			app.world()
				.resource::<Localization<TestCatalog, Lazy>>()
				.status::<TestCatalog>(),
			ModuleStatus::Unloaded
		);

		{
			let mut controller = app
				.world_mut()
				.resource_mut::<Localization<TestCatalog, Lazy>>();
			controller.load::<TestCatalog>();
			controller.load::<TestCatalog>();
		}

		app.update();
		assert_eq!(app.world().get::<Text>(label).unwrap().0, "ja");
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.set_locale("es");
		assert!(
			app.world()
				.resource::<Localization<TestCatalog, Lazy>>()
				.catalog()
				.is_none()
		);
		app.update();
		assert_eq!(app.world().get::<Text>(label).unwrap().0, "es");
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.unload::<TestCatalog>();
		app.update();
		assert!(!app.world().contains_resource::<TestCatalog>());
		assert!(app.world().get::<Text>(label).unwrap().0.is_empty());
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.load::<TestCatalog>();
		app.update();
		assert_eq!(app.world().get::<Text>(label).unwrap().0, "es");
	}
}

#[test]
#[should_panic(expected = "one localization plugin per root provider")]
fn two_loading_modes_cannot_race_for_the_same_resources() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<TestCatalog, Full>::new(manifest()),
		LocalizationPlugin::<TestCatalog, Lazy>::new(manifest()),
	));
}

#[test]
fn manifest_startup_policy_selects_a_known_locale_and_schema_mismatch_rejects() {
	let mut app = App::new();
	let source =
		crate::LocalizationManifest::__embedded(("ja", "es", ".", &[("es", "ui.ftl", b"es")]));
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<TestCatalog>::new(source),
	));
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().locale(),
		"es"
	);
	app.finish();
	app.cleanup();
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");

	let mut rejected = App::new();
	let source = crate::LocalizationManifest::__embedded((
		"de",
		"ja",
		".",
		&[("ja", "ui.ftl", b"wrong schema source")],
	));
	rejected.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<TestCatalog>::new(source),
	));
	rejected.finish();
	rejected.cleanup();
	rejected.update();
	assert!(!rejected.world().contains_resource::<TestCatalog>());
	assert!(matches!(
		rejected
			.world()
			.resource::<Localization<TestCatalog>>()
			.status::<TestCatalog>(),
		ModuleStatus::Failed(_)
	));
}

#[test]
fn prepared_file_locale_waits_for_retirement_and_survives_canceled_late_reads() {
	for inside_loader in [false, true] {
		let (mut app, gate, files) = asynchronous_app();
		gate.pass_new_reads.store(true, Ordering::SeqCst);
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.load::<TestCatalog>();
		pump(&mut app, |world| world.contains_resource::<TestCatalog>());
		gate.pass_new_reads.store(false, Ordering::SeqCst);
		gate.in_loader.store(inside_loader, Ordering::SeqCst);
		files.insert_asset_text(Path::new("nested/data/es/ui.ftl"), "old target");
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.prepare_locale("es");
		pump(&mut app, |_| !gate.state.lock().unwrap().2.is_empty());
		let first_request = app
			.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation
			.as_ref()
			.unwrap()
			.entries["ui.ftl"]
			.preparation_request;
		assert!(first_request.is_some());
		assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
		{
			let mut localization = app
				.world_mut()
				.resource_mut::<Localization<TestCatalog, Lazy>>();
			localization.cancel_preparation();
			localization.prepare_locale("de");
			localization.prepare_locale("es");
		}
		files.insert_asset_text(Path::new("nested/data/es/ui.ftl"), "fresh target");
		gate.pass_new_reads.store(true, Ordering::SeqCst);
		app.update();
		assert_eq!(
			app.world()
				.resource::<Localization<TestCatalog, Lazy>>()
				.preparation_status(),
			crate::PreparationStatus::Preparing
		);
		gate.release();
		pump(&mut app, |world| {
			world
				.resource::<Localization<TestCatalog, Lazy>>()
				.preparation_status()
				== crate::PreparationStatus::Ready
		});
		let second_request = app
			.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation
			.as_ref()
			.unwrap()
			.entries["ui.ftl"]
			.preparation_request;
		assert_ne!(first_request, second_request);
		assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.commit_locale()
			.unwrap();
		app.update();
		assert_eq!(app.world().resource::<TestCatalog>().0, "fresh target");
		gate.release();
		for _ in 0..20 {
			app.update();
			std::thread::sleep(Duration::from_millis(2));
			assert_eq!(app.world().resource::<TestCatalog>().0, "fresh target");
		}
		files.insert_asset_text(Path::new("nested/data/es/ui.ftl"), "active reload");
		request_reload(app.world_mut());
		pump(&mut app, |world| {
			world.resource::<TestCatalog>().0 == "active reload"
		});
		assert_eq!(
			app.world()
				.resource::<Localization<TestCatalog, Lazy>>()
				.locale(),
			"es"
		);
	}
}

#[test]
fn failed_target_file_attempt_and_retry_leave_active_snapshot_usable() {
	for corrupt in [false, true] {
		let (mut app, gate, files) = asynchronous_app();
		gate.pass_new_reads.store(true, Ordering::SeqCst);
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.load::<TestCatalog>();
		pump(&mut app, |world| world.contains_resource::<TestCatalog>());
		if corrupt {
			files.insert_asset(Path::new("nested/data/es/ui.ftl"), vec![0xff]);
		} else {
			files.remove_asset(Path::new("nested/data/es/ui.ftl"));
		}
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.prepare_locale("es");
		pump(&mut app, |world| {
			matches!(
				world
					.resource::<Localization<TestCatalog, Lazy>>()
					.preparation_status(),
				crate::PreparationStatus::Failed(_)
			)
		});
		if !corrupt {
			// Acquisition failed before settings could be applied. Its source
			// failure event remains authoritative over the completion fallback.
			pump(
				&mut app,
				|world| matches!(world.resource::<Localization<TestCatalog, Lazy>>().preparation_status(), crate::PreparationStatus::Failed(ref failure) if matches!(failure.status, ModuleStatus::Failed(ref error) if !error.contains("expected catalog loader"))),
			);
			for _ in 0..3 {
				app.update();
			}
			assert!(
				matches!(app.world().resource::<Localization<TestCatalog, Lazy>>().preparation_status(), crate::PreparationStatus::Failed(ref failure) if matches!(failure.status, ModuleStatus::Failed(ref error) if !error.contains("expected catalog loader")))
			);
		}

		assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
		assert!(matches!(
			app.world_mut()
				.resource_mut::<Localization<TestCatalog, Lazy>>()
				.commit_locale(),
			Err(crate::CommitLocaleError::Failed(_))
		));
		files.insert_asset_text(Path::new("nested/data/es/ui.ftl"), "repaired");
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.prepare_locale("es");
		pump(&mut app, |world| {
			world
				.resource::<Localization<TestCatalog, Lazy>>()
				.preparation_status()
				== crate::PreparationStatus::Ready
		});
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.commit_locale()
			.unwrap();
		app.update();
		assert_eq!(app.world().resource::<TestCatalog>().0, "repaired");
	}
}

#[test]
fn canceled_manifest_preparation_releases_private_assets_after_io_settles() {
	let (mut app, gate, _) = asynchronous_app();
	gate.pass_new_reads.store(true, Ordering::SeqCst);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	gate.pass_new_reads.store(false, Ordering::SeqCst);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.prepare_locale("es");
	pump(&mut app, |_| !gate.state.lock().unwrap().2.is_empty());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.cancel_preparation();
	gate.release();
	pump(&mut app, |world| {
		world
			.resource::<Assets<crate::assets::ModuleAsset<TestCatalog>>>()
			.iter()
			.all(|(_, asset)| asset.locale == "ja")
	});
	// Let detached loader completion reach Bevy and run handle retirement too.
	for _ in 0..30 {
		app.update();
		std::thread::sleep(Duration::from_millis(2));
	}
	assert_eq!(
		app.world()
			.resource::<Assets<crate::assets::ModuleAsset<TestCatalog>>>()
			.len(),
		1
	);
	assert!(
		app.world()
			.resource::<Assets<crate::assets::PreparedModuleAsset<TestCatalog>>>()
			.is_empty()
	);
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
}

#[test]
fn incompatible_explicit_loader_metadata_fails_preparation_and_can_recover() {
	let (mut app, gate, files) = asynchronous_app();
	gate.pass_new_reads.store(true, Ordering::SeqCst);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	let loader = std::any::type_name::<crate::assets::ModuleLoader<TestCatalog>>();
	let path = Path::new("nested/data/es/ui.ftl");
	files.insert_meta_text(
		path,
		&format!("(meta_format_version: \"1.0\", asset: Load(loader: \"{loader}\", settings: ()))"),
	);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog, Lazy>>()
				.preparation_status(),
			crate::PreparationStatus::Failed(_)
		)
	});
	let error = app
		.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.commit_locale()
		.unwrap_err();
	assert!(
		matches!(&error, crate::CommitLocaleError::Failed(module) if module.path == "ui.ftl" && module.locale == "es")
	);
	assert!(std::error::Error::source(&error).is_some());
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.locale(),
		"ja"
	);
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19", feature = "bevy-0-20"))]
	let repaired_locale = {
		files.remove_metadata(path);
		"es"
	};
	// Older memory fixtures expose no metadata removal API; recover by replacing
	// the target. Same-target metadata repair is covered on the newer backends.
	#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17"))]
	let repaired_locale = "de";
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.prepare_locale(repaired_locale);
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation_status()
			== crate::PreparationStatus::Ready
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, repaired_locale);
}

#[test]
fn unloading_during_retry_retirement_releases_unrequested_target_snapshots() {
	let (mut app, gate, _) = asynchronous_app();
	gate.pass_new_reads.store(true, Ordering::SeqCst);
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
			== crate::PreparationStatus::Ready
	});
	request_reload(app.world_mut());
	let mut once = true;
	app.add_systems(
		Update,
		move |mut state: ResMut<Localization<TestCatalog, Lazy>>| {
			if !once {
				return;
			}

			once = false;
			assert!(state.preparation.as_ref().unwrap().entries.is_empty());
			assert!(state.preparation.as_ref().unwrap().catalog().is_some());
			state.unload::<TestCatalog>();
			assert!(state.preparation.as_ref().unwrap().catalog().is_none());
			assert_eq!(state.preparation_status(), crate::PreparationStatus::Ready);
			state.commit_locale().unwrap();
		},
	);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	app.update();
	let state = app.world().resource::<Localization<TestCatalog, Lazy>>();
	assert_eq!(state.locale(), "es");
	assert!(state.desired().is_empty());
	assert!(state.catalog().is_none());
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn target_handoff_waits_for_an_obsolete_normal_reader_and_preserves_fresh_commit() {
	let (mut app, gate, files) = asynchronous_configured();
	app.add_plugins(LocalizationProgressPlugin::<TestCatalog>::default());
	app.finish();
	app.cleanup();
	gate.pass_new_reads.store(true, Ordering::SeqCst);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	gate.pass_new_reads.store(false, Ordering::SeqCst);
	files.insert_asset_text(Path::new("nested/data/es/ui.ftl"), "obsolete es read");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.set_locale("es");
	pump(&mut app, |_| !gate.state.lock().unwrap().2.is_empty());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.set_locale("ja");
	gate.pass_new_reads.store(true, Ordering::SeqCst);
	pump(&mut app, |world| {
		world
			.get_resource::<TestCatalog>()
			.is_some_and(|text| text.0 == "ja")
	});
	files.insert_asset_text(Path::new("nested/data/es/ui.ftl"), "fresh prepared es");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation
			.as_ref()
			.unwrap()
			.store
			.status::<TestCatalog>()
			== ModuleStatus::Ready
	});
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation_status(),
		crate::PreparationStatus::Preparing
	);
	assert_eq!(
		app.world_mut()
			.resource_mut::<Localization<TestCatalog, Lazy>>()
			.commit_locale(),
		Err(crate::CommitLocaleError::Pending)
	);
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	{
		let progress = app
			.world()
			.resource::<crate::LocalizationProgress<TestCatalog>>();
		let target = progress.preparation().unwrap();
		assert_eq!(target.ready, target.total);
		assert_eq!(target.total, 1);
		assert_eq!(
			progress.preparation_status(),
			&crate::PreparationStatus::Preparing
		);
		let tick = app
			.world()
			.get_resource_ref::<crate::LocalizationProgress<TestCatalog>>()
			.unwrap()
			.last_changed();

		for _ in 0..10 {
			app.update();
		}

		assert_eq!(
			app.world()
				.get_resource_ref::<crate::LocalizationProgress<TestCatalog>>()
				.unwrap()
				.last_changed(),
			tick
		);
	}

	gate.release();
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Lazy>>()
			.preparation_status()
			== crate::PreparationStatus::Ready
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(app.world().resource::<TestCatalog>().0, "fresh prepared es");
	for _ in 0..20 {
		app.update();
		std::thread::sleep(Duration::from_millis(1));
		assert_eq!(app.world().resource::<TestCatalog>().0, "fresh prepared es");
	}
}
