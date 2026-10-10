//! Native progress observes loading without establishing demand or idle UI work.
use super::{Gate, TestCatalog, pump, reload};
use crate::bevy::{ecs::schedule::common_conditions::resource_changed, prelude::*};
use crate::{
	Full, LoadingMode, Localization, LocalizationPlugin, LocalizationProgress,
	LocalizationProgressPlugin, PreparationStatus,
};
use std::sync::{
	Arc, Mutex,
	atomic::{AtomicBool, AtomicUsize, Ordering},
};

type Progress = LocalizationProgress<TestCatalog>;

#[derive(Clone, Default)]
struct Source {
	requests: Arc<Mutex<Vec<Arc<Gate>>>>,
	fail: Arc<AtomicBool>,
}

impl Source {
	fn count(&self) -> usize {
		self.requests.lock().unwrap().len()
	}

	fn release(&self, index: usize) {
		self.requests.lock().unwrap()[index].release();
	}
}

fn controlled<M: LoadingMode>() -> (App, Source, Arc<AtomicUsize>) {
	let source = Source::default();
	let observed = source.clone();
	let plugin = LocalizationPlugin::<TestCatalog, M>::from_loader(move |locale, _| {
		let gate = Arc::new(Gate::default());
		observed.requests.lock().unwrap().push(gate.clone());
		let fail = observed.fail.load(Ordering::Acquire);

		async move {
			gate.wait().await;

			if fail {
				Err("offline".to_owned())
			} else {
				Ok(locale.as_bytes().to_vec())
			}
		}
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin))
		.add_plugins(LocalizationProgressPlugin::<TestCatalog>::new());
	let calls = Arc::new(AtomicUsize::new(0));
	let observer = calls.clone();
	app.add_systems(
		Update,
		(move |_: Res<Progress>| {
			observer.fetch_add(1, Ordering::Relaxed);
		})
		.run_if(resource_changed::<Progress>),
	);
	app.finish();
	app.cleanup();
	assert!(app.world().contains_resource::<Progress>());
	(app, source, calls)
}

fn assert_idle(app: &mut App, calls: &AtomicUsize) {
	app.update();
	app.update();
	let before = calls.load(Ordering::Relaxed);
	let tick = app
		.world()
		.get_resource_ref::<Progress>()
		.unwrap()
		.last_changed();

	for _ in 0..20 {
		app.update();
	}

	assert_eq!(calls.load(Ordering::Relaxed), before);
	assert_eq!(
		app.world()
			.get_resource_ref::<Progress>()
			.unwrap()
			.last_changed(),
		tick
	);
}

#[test]
fn full_loading_and_ordinary_locale_selection_publish_without_idle_observer_runs() {
	let (mut app, source, calls) = controlled::<Full>();
	let active = app.world().resource::<Progress>().active();
	assert_eq!(active.total, 1);
	assert_eq!(active.unloaded, 1);
	assert_eq!(active.available, 0);
	pump(&mut app, |_| source.count() == 1);
	assert_eq!(app.world().resource::<Progress>().active().loading, 1);
	assert_idle(&mut app, &calls);
	source.release(0);
	pump(&mut app, |world| {
		world.resource::<Progress>().active().ready == 1
	});
	assert_eq!(app.world().resource::<Progress>().active().available, 1);
	assert_idle(&mut app, &calls);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |_| source.count() == 2);
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.active().locale, "es");
	assert_eq!(progress.active().loading, 1);
	assert_eq!(progress.active().available, 0);
	assert!(progress.preparation().is_none());
	assert!(!app.world().contains_resource::<TestCatalog>());
	source.release(1);
	pump(&mut app, |world| {
		world.resource::<Progress>().active().ready == 1
	});
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");
	assert_idle(&mut app, &calls);
}

#[test]
fn prepare_failure_retry_cancel_and_commit_keep_active_progress_independent() {
	let (mut app, source, calls) = controlled::<Full>();
	pump(&mut app, |_| source.count() == 1);
	source.release(0);
	pump(&mut app, |world| {
		world.resource::<Progress>().active().ready == 1
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("ja");
	app.update();
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.preparation().unwrap(), progress.active());
	assert_eq!(progress.preparation_status(), &PreparationStatus::Ready);
	assert_eq!(source.count(), 1);
	#[cfg(feature = "diagnostics")]
	{
		let state = app.world().resource::<Localization<TestCatalog>>();
		assert_eq!(
			state.preparation_diagnostics::<TestCatalog>().unwrap(),
			state.diagnostics::<TestCatalog>()
		);
	}
	assert_idle(&mut app, &calls);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.cancel_preparation();
	app.update();
	assert!(app.world().resource::<Progress>().preparation().is_none());

	source.fail.store(true, Ordering::Release);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("es");
	pump(&mut app, |_| source.count() == 2);
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.active().locale, "ja");
	assert_eq!(progress.active().available, 1);
	assert_eq!(progress.preparation().unwrap().locale, "es");
	assert_eq!(progress.preparation().unwrap().loading, 1);
	assert_idle(&mut app, &calls);
	source.release(1);
	pump(&mut app, |world| {
		matches!(
			world.resource::<Progress>().preparation_status(),
			PreparationStatus::Failed(_)
		)
	});
	assert_eq!(
		app.world()
			.resource::<Progress>()
			.preparation()
			.unwrap()
			.failed,
		1
	);
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	#[cfg(feature = "diagnostics")]
	{
		let state = app.world().resource::<Localization<TestCatalog>>();
		assert_eq!(
			state.diagnostics::<TestCatalog>()[0].status,
			crate::ModuleStatus::Ready
		);
		let target = state.preparation_diagnostics::<TestCatalog>().unwrap();
		assert!(matches!(target[0].status, crate::ModuleStatus::Failed(_)));
		assert!(!target[0].usable);
	}
	assert_idle(&mut app, &calls);

	source.fail.store(false, Ordering::Release);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("es");
	pump(&mut app, |_| source.count() == 3);
	assert_eq!(
		app.world()
			.resource::<Progress>()
			.preparation()
			.unwrap()
			.loading,
		1
	);
	source.release(2);
	pump(&mut app, |world| {
		matches!(
			world.resource::<Progress>().preparation_status(),
			PreparationStatus::Ready
		)
	});
	assert_eq!(
		app.world()
			.resource::<Progress>()
			.preparation()
			.unwrap()
			.ready,
		1
	);
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	assert_idle(&mut app, &calls);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.commit_locale()
		.unwrap();
	app.update();
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.active().locale, "es");
	assert_eq!(progress.active().ready, 1);
	assert!(progress.preparation().is_none());
	assert_eq!(progress.preparation_status(), &PreparationStatus::Idle);
	#[cfg(feature = "diagnostics")]
	assert!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.preparation_diagnostics::<TestCatalog>()
			.is_none()
	);
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");
	assert_idle(&mut app, &calls);
}

#[test]
fn failed_active_reload_retains_availability_and_controller_replacement_resets_it() {
	let (mut app, source, calls) = controlled::<Full>();
	pump(&mut app, |_| source.count() == 1);
	source.release(0);
	pump(&mut app, |world| {
		world.resource::<Progress>().active().ready == 1
	});
	source.fail.store(true, Ordering::Release);
	reload(&mut app);
	pump(&mut app, |_| source.count() == 2);
	let active = app.world().resource::<Progress>().active();
	assert_eq!(active.loading, 1);
	assert_eq!(active.available, 1);
	assert_idle(&mut app, &calls);
	source.release(1);
	pump(&mut app, |world| {
		world.resource::<Progress>().active().failed == 1
	});
	assert_eq!(app.world().resource::<Progress>().active().available, 1);
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
	#[cfg(feature = "diagnostics")]
	{
		let state = app.world().resource::<Localization<TestCatalog>>();
		let modules = state.diagnostics::<TestCatalog>();
		assert!(matches!(modules[0].status, crate::ModuleStatus::Failed(_)));
		assert!(modules[0].usable);
		assert!(state.preparation_diagnostics::<TestCatalog>().is_none());
	}
	assert_idle(&mut app, &calls);

	source.fail.store(false, Ordering::Release);
	app.world_mut()
		.insert_resource(Localization::<TestCatalog>::default());
	pump(&mut app, |_| source.count() == 3);
	assert_eq!(app.world().resource::<Progress>().active().available, 0);
	assert_eq!(app.world().resource::<Progress>().active().loading, 1);
	source.release(2);
	pump(&mut app, |world| {
		world.resource::<Progress>().active().ready == 1
	});
	app.world_mut().remove_resource::<Progress>();
	app.update();
	assert_eq!(app.world().resource::<Progress>().active().ready, 1);
	assert_idle(&mut app, &calls);
}
