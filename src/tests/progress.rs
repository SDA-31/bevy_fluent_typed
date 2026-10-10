use super::TestCatalog;
#[cfg(feature = "diagnostics")]
use crate::ModuleStatus;
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{FluentScope, Lazy, Localization, LocalizationPlugin, ModuleStore, ReloadCatalogs};
use std::{
	sync::{Arc, Mutex},
	time::{Duration, Instant},
};

#[derive(Resource, Clone)]
struct Repeated;

impl FluentScope for Repeated {
	type Catalog = TestCatalog;

	fn module_paths() -> &'static [&'static str] {
		&["ui.ftl", "unknown.ftl", "ui.ftl"]
	}

	fn assemble(_: &ModuleStore<TestCatalog>) -> Option<Self> {
		None
	}
}

#[derive(Resource, Clone)]
struct Empty;

impl FluentScope for Empty {
	type Catalog = TestCatalog;

	fn module_paths() -> &'static [&'static str] {
		&[]
	}

	fn assemble(_: &ModuleStore<TestCatalog>) -> Option<Self> {
		Some(Self)
	}
}

#[test]
fn inspection_deduplicates_paths_and_does_not_request_unknown_or_empty_scopes() {
	let mut state = Localization::<TestCatalog, Lazy>::default();
	let progress = state.progress::<Repeated>();
	assert_eq!(progress.locale, "ja");
	assert_eq!(progress.total, 2);
	assert_eq!(progress.unloaded, 2);
	assert_eq!(progress.ready + progress.loading + progress.failed, 0);
	assert_eq!(progress.available, 0);
	#[cfg(feature = "diagnostics")]
	{
		let modules = state.diagnostics::<Repeated>();
		assert_eq!(modules.len(), 2);
		assert_eq!(modules[0].path, "ui.ftl");
		assert_eq!(modules[1].path, "unknown.ftl");
		assert!(
			modules
				.iter()
				.all(|module| module.status == ModuleStatus::Unloaded && !module.usable)
		);
		assert_eq!(state.modules().diagnostics::<Repeated>(), modules);
	}
	assert_eq!(state.modules().progress::<Repeated>(), progress);
	let empty = state.progress::<Empty>();
	assert_eq!(empty.total, 0);
	assert_eq!(
		empty.ready + empty.loading + empty.failed + empty.unloaded,
		0
	);
	assert_eq!(empty.available, 0);
	#[cfg(feature = "diagnostics")]
	assert!(state.diagnostics::<Empty>().is_empty());
	assert!(state.requested.is_empty());
	assert!(state.entries.is_empty());
	assert!(state.retry.is_empty());
	assert!(state.store.states.is_empty());
	assert_eq!(state.pending, 0);
	state
		.store
		.insert_leaf("ui.ftl", Arc::new(TestCatalog("ready".into())));
	let progress = state.progress::<Repeated>();
	assert_eq!(progress.total, 2);
	assert_eq!(progress.ready, 1);
	assert_eq!(progress.unloaded, 1);
	assert_eq!(progress.available, 1);
	#[cfg(feature = "diagnostics")]
	{
		let modules = state.diagnostics::<Repeated>();
		assert!(modules[0].usable);
		assert!(!modules[1].usable);
	}
}

fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if ready(app.world()) {
			return;
		}

		assert!(Instant::now() < deadline, "progress source timed out");
		std::thread::sleep(Duration::from_millis(1));
	}
}

#[test]
fn failed_reload_diagnostics_keep_errors_and_last_good_availability_until_unload() {
	let data = Arc::new(Mutex::new(Ok(b"first".to_vec())));
	let source = data.clone();
	let plugin = LocalizationPlugin::<TestCatalog, Lazy>::from_loader(move |_, _| {
		std::future::ready(source.lock().unwrap().clone())
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());

	for (candidate, _expected) in [
		(Err("offline".to_owned()), "load ja/ui.ftl: offline"),
		(Ok(vec![0xff]), "parse ja/ui.ftl:"),
	] {
		*data.lock().unwrap() = candidate;
		#[cfg(feature = "bevy-0-16")]
		app.world_mut()
			.send_event(ReloadCatalogs::<TestCatalog>::default());
		#[cfg(not(feature = "bevy-0-16"))]
		app.world_mut()
			.write_message(ReloadCatalogs::<TestCatalog>::default());
		pump(&mut app, |world| {
			world
				.resource::<Localization<TestCatalog, Lazy>>()
				.progress::<TestCatalog>()
				.failed == 1
		});
		let state = app.world().resource::<Localization<TestCatalog, Lazy>>();
		let progress = state.progress::<TestCatalog>();
		assert_eq!(progress.total, 1);
		assert_eq!(progress.failed, 1);
		assert_eq!(progress.ready + progress.loading + progress.unloaded, 0);
		assert_eq!(progress.available, 1);
		#[cfg(feature = "diagnostics")]
		{
			let modules = state.diagnostics::<TestCatalog>();
			assert_eq!(modules[0].path, "ui.ftl");
			assert_eq!(modules[0].status, state.status::<TestCatalog>());
			assert!(modules[0].usable);
			let ModuleStatus::Failed(error) = &modules[0].status else {
				panic!("failed attempt must preserve its error");
			};
			assert!(error.starts_with(_expected));
		}
		assert_eq!(state.modules().get::<TestCatalog>().unwrap().0, "first");
	}

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.unload::<TestCatalog>();
	app.update();
	let progress = app
		.world()
		.resource::<Localization<TestCatalog, Lazy>>()
		.progress::<TestCatalog>();
	assert_eq!(progress.total, 1);
	assert_eq!(progress.unloaded, 1);
	assert_eq!(progress.failed, 0);
	assert_eq!(progress.available, 0);
	#[cfg(feature = "diagnostics")]
	assert!(
		!app.world()
			.resource::<Localization<TestCatalog, Lazy>>()
			.diagnostics::<TestCatalog>()[0]
			.usable
	);
}
