use bevy_fluent_typed::bevy::{
	ecs::{self as bevy_ecs, schedule::common_conditions::resource_changed},
	prelude::*,
};
use bevy_fluent_typed::{
	CatalogUpdate, CatalogUpdateReader, Localization, LocalizationPlugin, LocalizationProgress,
	LocalizationProgressPlugin, LocalizationSystems, PreparationStatus,
};
use std::{
	sync::{Arc, Mutex},
	task::{Poll, Waker},
	time::{Duration, Instant},
};

bevy_fluent_typed::translations!(mod texts);

type Progress = LocalizationProgress<texts::Translations>;
type GreetingProgress = LocalizationProgress<texts::ui::Greeting>;
type Attempts = Arc<Mutex<Vec<(texts::Locale, Arc<Gate>)>>>;

#[derive(Default)]
struct Gate(Mutex<(bool, Option<Waker>)>);

impl Gate {
	async fn wait(&self) {
		std::future::poll_fn(|context| {
			let mut state = self.0.lock().unwrap();

			if state.0 {
				Poll::Ready(())
			} else {
				state.1 = Some(context.waker().clone());
				Poll::Pending
			}
		})
		.await
	}

	fn release(&self) {
		let mut state = self.0.lock().unwrap();
		state.0 = true;

		if let Some(waker) = state.1.take() {
			waker.wake();
		}
	}
}

#[derive(Resource, Default)]
struct Observations {
	updates: usize,
	rejections: Vec<Option<texts::Locale>>,
}

fn observe(progress: Res<Progress>, mut observations: ResMut<Observations>) {
	assert_eq!(progress.active().total, 1);
	observations.updates += 1;
}

fn record(mut updates: CatalogUpdateReader<texts::Translations>, mut seen: ResMut<Observations>) {
	for update in updates.read() {
		if let CatalogUpdate::Rejected { locale, .. } = update {
			seen.rejections.push(*locale);
		}
	}
}

fn controlled_app(fail_target: bool) -> (App, Attempts) {
	let attempts = Attempts::default();
	let observed = attempts.clone();
	let plugin = LocalizationPlugin::<texts::Translations>::from_loader(move |locale, path| {
		assert_eq!(path, texts::ui::Greeting::PATH);
		let gate = Arc::new(Gate::default());
		observed.lock().unwrap().push((locale, gate.clone()));

		async move {
			gate.wait().await;

			if fail_target && locale == texts::Locale::Es {
				return Err("target unavailable".to_string());
			}

			let bytes = match locale {
				texts::Locale::En => b"hello = Hello!".as_slice(),
				texts::Locale::Es => "hello = ¡Hola!".as_bytes(),
				texts::Locale::Ru => "hello = Привет!".as_bytes(),
			};

			Ok(bytes.to_vec())
		}
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin))
		.add_plugins(LocalizationProgressPlugin::<texts::Translations>::default())
		.add_plugins(LocalizationProgressPlugin::<texts::ui::Greeting>::default())
		.init_resource::<Observations>()
		.add_systems(Startup, |progress: Res<Progress>| {
			assert_eq!(progress.active().unloaded, 1);
		})
		.add_systems(Update, record)
		.add_systems(
			PostUpdate,
			observe
				.run_if(resource_changed::<Progress>)
				.after(LocalizationSystems::Progress),
		);
	app.finish();
	app.cleanup();
	assert_eq!(app.world().resource::<Progress>().active().unloaded, 1);
	assert_eq!(
		app.world().resource::<GreetingProgress>().active().unloaded,
		1
	);
	(app, attempts)
}

fn release(attempts: &Attempts, locale: texts::Locale) {
	attempts
		.lock()
		.unwrap()
		.iter()
		.find(|(language, _)| *language == locale)
		.unwrap()
		.1
		.release();
}

fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if ready(app.world()) {
			return;
		}

		assert!(Instant::now() < deadline, "native progress test timed out");
		std::thread::sleep(Duration::from_millis(1));
	}
}

fn load_initial(app: &mut App, attempts: &Attempts) {
	pump(app, |world| {
		world.resource::<Progress>().active().loading == 1 && attempts.lock().unwrap().len() == 1
	});
	release(attempts, texts::Locale::En);
	pump(app, |world| {
		world.resource::<Progress>().active().ready == 1
	});
	assert_eq!(
		app.world().resource::<Progress>().active(),
		app.world().resource::<GreetingProgress>().active()
	);
	assert!(app.world().resource::<Observations>().updates > 0);
}

fn assert_idle(app: &mut App) {
	let before = app.world().resource::<Observations>().updates;
	let tick = app
		.world()
		.get_resource_ref::<Progress>()
		.unwrap()
		.last_changed();

	for _ in 0..5 {
		app.update();
		assert_eq!(app.world().resource::<Observations>().updates, before);
		assert_eq!(
			app.world()
				.get_resource_ref::<Progress>()
				.unwrap()
				.last_changed(),
			tick
		);
	}
}

#[test]
fn generated_full_loading_and_explicit_commit_publish_native_progress_without_idle_updates() {
	let (mut app, attempts) = controlled_app(false);
	load_initial(&mut app, &attempts);
	assert_eq!(
		app.world().resource::<texts::ui::Greeting>().msg_hello(),
		"Hello!"
	);
	assert_idle(&mut app);
	let before = app.world().resource::<Observations>().updates;
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.prepare_locale(texts::Locale::Es);
	pump(&mut app, |world| {
		world
			.resource::<Progress>()
			.preparation()
			.is_some_and(|target| target.loading == 1)
			&& attempts.lock().unwrap().len() == 2
	});
	assert!(app.world().resource::<Observations>().updates > before);
	assert_eq!(
		app.world().resource::<Progress>().active().locale,
		texts::Locale::En
	);
	release(&attempts, texts::Locale::Es);
	pump(&mut app, |world| {
		world.resource::<Progress>().preparation_status() == &PreparationStatus::Ready
	});
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.active().available, 1);
	assert_eq!(progress.preparation().unwrap().ready, 1);
	assert_eq!(
		app.world().resource::<texts::ui::Greeting>().msg_hello(),
		"Hello!"
	);
	assert_idle(&mut app);
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.commit_locale()
		.unwrap();
	pump(&mut app, |world| {
		let progress = world.resource::<Progress>();

		progress.active().locale == texts::Locale::Es
			&& progress.active().ready == 1
			&& progress.preparation().is_none()
	});
	assert_eq!(
		app.world().resource::<Progress>().preparation_status(),
		&PreparationStatus::Idle
	);
	assert_eq!(
		app.world().resource::<texts::ui::Greeting>().msg_hello(),
		"¡Hola!"
	);
	assert_idle(&mut app);
}

#[test]
fn failed_manual_preparation_is_visible_without_replacing_active_data_or_emitting_rejection() {
	let (mut app, attempts) = controlled_app(true);
	load_initial(&mut app, &attempts);
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.prepare_locale(texts::Locale::Es);
	pump(&mut app, |_| attempts.lock().unwrap().len() == 2);
	release(&attempts, texts::Locale::Es);
	pump(&mut app, |world| {
		matches!(
			world.resource::<Progress>().preparation_status(),
			PreparationStatus::Failed(_)
		)
	});
	let progress = app.world().resource::<Progress>();
	assert_eq!(progress.active().locale, texts::Locale::En);
	assert_eq!(progress.active().ready, 1);
	assert_eq!(progress.active().available, 1);
	assert_eq!(progress.preparation().unwrap().failed, 1);
	assert_eq!(progress.preparation().unwrap().available, 0);
	assert_eq!(
		app.world().resource::<texts::ui::Greeting>().msg_hello(),
		"Hello!"
	);
	assert_idle(&mut app);
	assert!(app.world().resource::<Observations>().rejections.is_empty());
	let before = app.world().resource::<Observations>().updates;
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.cancel_preparation();
	pump(&mut app, |world| {
		world.resource::<Progress>().preparation().is_none()
	});
	assert!(app.world().resource::<Observations>().updates > before);
	assert_eq!(
		app.world().resource::<Progress>().preparation_status(),
		&PreparationStatus::Idle
	);
	assert_eq!(
		app.world().resource::<texts::ui::Greeting>().msg_hello(),
		"Hello!"
	);
	assert_idle(&mut app);
}
