use super::TestCatalog;
use crate::bevy::prelude::*;
use crate::{
	Localization, LocalizationAppExt, LocalizationPlugin, LocalizedText, Manual, ModuleStatus,
};
use std::{
	any::TypeId,
	sync::{
		Arc, Mutex,
		atomic::{AtomicUsize, Ordering},
	},
	task::{Poll, Waker},
	time::{Duration, Instant},
};

mod progress;
mod tree;

pub(super) fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
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

fn app<M: crate::LoadingMode>(plugin: LocalizationPlugin<TestCatalog, M>) -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();
	assert!(!app.world().contains_resource::<AssetServer>());
	app
}

fn reload(app: &mut App) {
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(crate::ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
}

#[test]
fn bytes_publish_checked_resources_and_bindings_without_asset_plugin() {
	let plugin = LocalizationPlugin::<TestCatalog>::from_bytes([
		("ja", "ui.ftl", b"Japanese".as_slice()),
		("es", "ui.ftl", b"Spanish".as_slice()),
	])
	.unwrap();
	let mut app = app(plugin);
	let label = app
		.world_mut()
		.spawn((
			Text::default(),
			LocalizedText::<TestCatalog>::new(|text| text.0.clone()),
		))
		.id();
	let runs = Arc::new(AtomicUsize::new(0));
	let observed = runs.clone();
	app.add_localized_systems(Update, move |text: Res<TestCatalog>| {
		assert!(!text.0.is_empty());
		observed.fetch_add(1, Ordering::Relaxed);
	});

	for (locale, value) in [("ja", "Japanese"), ("es", "Spanish"), ("ja", "Japanese")] {
		let before = runs.load(Ordering::Relaxed);
		app.world_mut()
			.resource_mut::<Localization<TestCatalog>>()
			.set_locale(locale);
		pump(&mut app, |world| {
			world
				.get_resource::<TestCatalog>()
				.is_some_and(|text| text.0 == value)
				&& runs.load(Ordering::Relaxed) > before
		});
		assert_eq!(app.world().get::<Text>(label).unwrap().0, value);
	}

	assert!(runs.load(Ordering::Relaxed) > 0);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("de");
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog>>()
				.preparation_status(),
			crate::PreparationStatus::Failed(_)
		)
	});
	assert_eq!(app.world().resource::<TestCatalog>().0, "Japanese");
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "Japanese");
}

#[test]
fn byte_source_rejects_unknown_and_duplicate_keys() {
	for modules in [
		vec![("xx", "ui.ftl", b"data".as_slice())],
		vec![("ja", "typo.ftl", b"data".as_slice())],
		vec![
			("ja", "ui.ftl", b"one".as_slice()),
			("ja", "ui.ftl", b"two".as_slice()),
		],
	] {
		assert!(LocalizationPlugin::<TestCatalog>::from_bytes(modules).is_err());
	}
}

#[derive(Default)]
struct Gate(Mutex<(bool, Option<Waker>)>);

impl Gate {
	async fn wait(&self) {
		std::future::poll_fn(|cx| {
			let mut state = self.0.lock().unwrap();

			if state.0 {
				Poll::Ready(())
			} else {
				state.1 = Some(cx.waker().clone());
				Poll::Pending
			}
		})
		.await;
	}

	fn release(&self) {
		let mut state = self.0.lock().unwrap();
		state.0 = true;

		if let Some(waker) = state.1.take() {
			waker.wake();
		}
	}
}

type Requests = Arc<Mutex<Vec<(&'static str, Arc<Gate>)>>>;

fn controlled() -> (App, Requests) {
	let requests = Requests::default();
	let observed = requests.clone();
	let plugin = LocalizationPlugin::<TestCatalog, Manual>::from_loader(move |locale, path| {
		assert_eq!(path, "ui.ftl");
		let gate = Arc::new(Gate::default());
		observed.lock().unwrap().push((locale, gate.clone()));

		async move {
			gate.wait().await;
			Ok::<_, String>(locale.as_bytes().to_vec())
		}
	});
	(app(plugin), requests)
}

#[test]
fn manual_loader_waits_for_demand_and_discards_old_locale_completion() {
	let (mut app, requests) = controlled();
	let runs = Arc::new(AtomicUsize::new(0));
	let observed = runs.clone();
	app.add_localized_systems(Update, move |_: Res<TestCatalog>| {
		observed.fetch_add(1, Ordering::Relaxed);
	});

	for _ in 0..3 {
		app.update();
	}

	assert!(requests.lock().unwrap().is_empty());
	assert_eq!(runs.load(Ordering::Relaxed), 0);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| requests.lock().unwrap().len() == 1);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	app.update();
	assert_eq!(requests.lock().unwrap().len(), 1);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.set_locale("es");
	pump(&mut app, |_| requests.lock().unwrap().len() == 2);
	requests.lock().unwrap()[0].1.release();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	requests.lock().unwrap()[1].1.release();
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Manual>>()
			.locale() == "es"
	});
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");

	for _ in 0..3 {
		app.update();
		assert_eq!(app.world().resource::<TestCatalog>().0, "es");
	}
}

#[test]
fn unloaded_request_cannot_satisfy_a_new_request_for_the_same_leaf() {
	let (mut app, requests) = controlled();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| requests.lock().unwrap().len() == 1);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	app.update();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| requests.lock().unwrap().len() == 2);
	requests.lock().unwrap()[0].1.release();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	requests.lock().unwrap()[1].1.release();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
}

#[test]
fn pending_reload_requests_coalesce_into_one_fresh_fetch() {
	let (mut app, requests) = controlled();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |_| requests.lock().unwrap().len() == 1);

	for _ in 0..3 {
		reload(&mut app);
		app.update();
	}

	assert_eq!(requests.lock().unwrap().len(), 1);
	requests.lock().unwrap()[0].1.release();
	pump(&mut app, |_| requests.lock().unwrap().len() == 2);
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	requests.lock().unwrap()[1].1.release();
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Manual>>()
			.status::<TestCatalog>()
			== ModuleStatus::Ready
	});
	assert_eq!(requests.lock().unwrap().len(), 2);
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.retry
			.is_empty()
	);
}

#[test]
fn load_and_parse_failures_keep_last_good_data_and_can_retry() {
	let data = Arc::new(Mutex::new(Ok(b"first".to_vec())));
	let source = data.clone();
	let mut app = app(LocalizationPlugin::<TestCatalog, Manual>::from_loader(
		move |_, _| std::future::ready(source.lock().unwrap().clone()),
	));
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());

	for (candidate, expected) in [
		(Err("offline".to_owned()), "load ja/ui.ftl: offline"),
		(Ok(vec![0xff]), "parse ja/ui.ftl:"),
	] {
		*data.lock().unwrap() = candidate;
		reload(&mut app);
		pump(
			&mut app,
			|world| matches!(world.resource::<Localization<TestCatalog, Manual>>().status::<TestCatalog>(), ModuleStatus::Failed(ref error) if error.starts_with(expected)),
		);
		assert_eq!(app.world().resource::<TestCatalog>().0, "first");
	}

	*data.lock().unwrap() = Ok(b"repaired".to_vec());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world.resource::<TestCatalog>().0 == "repaired"
	});
}

struct Buffer {
	bytes: Vec<u8>,
	live: Arc<AtomicUsize>,
}

impl AsRef<[u8]> for Buffer {
	fn as_ref(&self) -> &[u8] {
		&self.bytes
	}
}

impl Drop for Buffer {
	fn drop(&mut self) {
		self.live.fetch_sub(1, Ordering::Relaxed);
	}
}

#[test]
fn loader_releases_input_after_parse_and_unload_releases_its_snapshot() {
	let live = Arc::new(AtomicUsize::new(0));
	let counter = live.clone();
	let mut app = app(LocalizationPlugin::<TestCatalog, Manual>::from_loader(
		move |_, _| {
			counter.fetch_add(1, Ordering::Relaxed);
			std::future::ready(Ok::<_, String>(Buffer {
				bytes: b"temporary".to_vec(),
				live: counter.clone(),
			}))
		},
	));
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(live.load(Ordering::Relaxed), 0);
	let weak = Arc::downgrade(
		&app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.store
			.values[&TypeId::of::<TestCatalog>()]
			.value,
	);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	app.update();
	assert!(weak.upgrade().is_none());
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.pending,
		0
	);
}
