//! Native progress observation with explicit control over locale publication.
use bevy_fluent_typed::bevy::{
	asset::AssetPlugin,
	ecs::{self as bevy_ecs, schedule::common_conditions::resource_changed},
	prelude::*,
};
use bevy_fluent_typed::{
	Localization, LocalizationAppExt, LocalizationPlugin, LocalizationProgress,
	LocalizationSystems, PreparationStatus,
};
use std::time::{Duration, Instant};

bevy_fluent_typed::translations!(mod texts);

const LOAD_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(1);

#[derive(Resource, Default)]
struct ProgressUi {
	updates: usize,
}

fn main() -> Result<(), String> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: env!("CARGO_MANIFEST_DIR").into(),
			..default()
		},
		LocalizationPlugin::<texts::Translations>::new(texts::manifest()),
	))
	.add_localization_progress::<texts::Translations>()
	.init_resource::<ProgressUi>()
	.add_systems(
		PostUpdate,
		observe_progress
			.run_if(resource_changed::<LocalizationProgress<texts::Translations>>)
			.after(LocalizationSystems::Progress),
	);
	app.finish();
	app.cleanup();
	wait(&mut app, |progress| progress.active().ready == 1)?;
	println!(
		"{}",
		app.world().resource::<texts::ui::Greeting>().msg_hello()
	);

	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.prepare_locale(texts::Locale::Es);
	wait(&mut app, |progress| {
		progress.preparation_status() == &PreparationStatus::Ready
	})?;
	assert_eq!(
		app.world().resource::<texts::ui::Greeting>().locale(),
		texts::Locale::En
	);
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.commit_locale()
		.map_err(|error| error.to_string())?;
	wait(&mut app, |progress| {
		progress.active().locale == texts::Locale::Es
			&& progress.active().ready == 1
			&& progress.preparation().is_none()
	})?;
	println!(
		"{}",
		app.world().resource::<texts::ui::Greeting>().msg_hello()
	);

	let before = app.world().resource::<ProgressUi>().updates;

	for _ in 0..5 {
		app.update();
	}

	assert_eq!(app.world().resource::<ProgressUi>().updates, before);
	Ok(())
}

fn observe_progress(
	progress: Res<LocalizationProgress<texts::Translations>>,
	mut ui: ResMut<ProgressUi>,
) {
	ui.updates += 1;
	let active = progress.active();
	eprintln!(
		"active {}: {}/{} ready, {} available",
		active.locale, active.ready, active.total, active.available
	);

	if let Some(target) = progress.preparation() {
		eprintln!(
			"target {}: {}/{} ready ({:?})",
			target.locale,
			target.ready,
			target.total,
			progress.preparation_status()
		);
	}
}

fn wait(
	app: &mut App,
	ready: impl Fn(&LocalizationProgress<texts::Translations>) -> bool,
) -> Result<(), String> {
	let deadline = Instant::now() + LOAD_TIMEOUT;

	loop {
		app.update();
		let progress = app
			.world()
			.resource::<LocalizationProgress<texts::Translations>>();

		if progress.active().failed > 0 {
			return Err(format!("active load failed: {:?}", progress.active()));
		}

		if let PreparationStatus::Failed(error) = progress.preparation_status() {
			return Err(error.to_string());
		}

		if ready(progress) {
			return Ok(());
		}

		if Instant::now() >= deadline {
			return Err("progress example timed out".into());
		}

		std::thread::sleep(POLL_INTERVAL);
	}
}
