//! Terminal-only runner for the external-load and live-edit demonstration.
use crate::texts::{Locale, Translations};
use localization_runtime::bevy::{ecs as bevy_ecs, prelude::*};
use localization_runtime::{CatalogUpdate, CatalogUpdateReader, Localization};
use std::time::{Duration, Instant};

// Bound failed external loads in one-shot mode; watch mode runs until interrupted.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Resource, Default)]
pub(super) struct LoadStatus {
	loaded: Vec<Locale>,
	failed: bool,
}

pub(super) fn run_until_loaded(
	app: &mut App,
	label: Entity,
	locale: Locale,
	watch: bool,
) -> Result<(), String> {
	app.world_mut().resource_mut::<LoadStatus>().failed = false;
	let deadline = Instant::now() + LOAD_TIMEOUT;
	let mut previous = String::new();

	// This console runner drives Bevy frames manually; normal apps use App::run().
	// Each update lets Bevy publish completed loads and refresh localized text.
	loop {
		app.update();
		let text = &app.world().get::<Text>(label).unwrap().0;

		if *text != previous {
			println!("{text}");
			previous.clone_from(text);
		}

		let status = app.world().resource::<LoadStatus>();

		if !watch {
			if status.failed {
				return Err("external catalogs failed; see diagnostic above".into());
			}

			if app
				.world()
				.resource::<Localization<Translations>>()
				.locale() == locale
				&& app.world().contains_resource::<crate::texts::Ui>()
			{
				break;
			}

			if Instant::now() >= deadline {
				return Err("timed out waiting for external catalogs".into());
			}
		}

		std::thread::sleep(POLL_INTERVAL);
	}

	Ok(())
}

pub(super) fn observe(
	mut updates: CatalogUpdateReader<Translations>,
	mut status: ResMut<LoadStatus>,
) {
	for update in updates.read() {
		match update {
			CatalogUpdate::Loaded { locale, .. } => {
				if !status.loaded.contains(locale) {
					status.loaded.push(*locale);
				}
			}
			CatalogUpdate::Rejected { path, error, .. } => {
				eprintln!("{path}: {error}");
				status.failed = true;
			}
		}
	}
}
