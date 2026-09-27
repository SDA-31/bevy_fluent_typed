use super::{TestCatalog, manifest};
use crate::bevy::asset::io::{
	AssetReader, AssetReaderError, AssetSourceBuilder, PathStream, Reader,
	memory::{Dir, MemoryAssetReader},
};
use crate::bevy::prelude::*;
use crate::bevy::tasks::futures_lite::io::AsyncRead;
use crate::{Full, Lazy, Localization, LocalizationPlugin, LocalizedText, ModuleStatus};
use std::{
	any::TypeId,
	path::Path,
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
	state: Mutex<(bool, Vec<String>, Vec<Waker>)>,
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

#[cfg(not(any(feature = "bevy-0-18", feature = "bevy-0-19")))]
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
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19"))]
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
		self.gate
			.state
			.lock()
			.unwrap()
			.1
			.push(path.to_string_lossy().into_owned());
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

fn asynchronous_app() -> (App, Arc<Gate>, Dir) {
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
	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19"))]
	let source = AssetSourceBuilder::new(reader);
	#[cfg(not(any(feature = "bevy-0-18", feature = "bevy-0-19")))]
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
	assert_eq!(paths, &["nested/data/ja/ui.ftl", "nested/data/es/ui.ftl"]);
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
