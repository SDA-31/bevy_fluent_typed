use super::{TestCatalog, bytes::pump};
use crate::bevy::prelude::*;
use crate::{
	CatalogUpdate, CatalogUpdateReader, Localization, LocalizationPlugin, LocalizedText,
	PreparationStatus,
};
use std::{
	sync::{
		Arc, Mutex,
		atomic::{AtomicUsize, Ordering},
	},
	task::{Poll, Waker},
};

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

type Attempts = Arc<Mutex<Vec<(&'static str, Arc<Gate>)>>>;

fn app() -> (App, Attempts, Entity) {
	let attempts = Attempts::default();
	let observed = attempts.clone();
	let plugin = LocalizationPlugin::<TestCatalog>::from_loader(move |locale, _| {
		let gate = Arc::new(Gate::default());

		if locale == "ja" {
			gate.release();
		}

		observed.lock().unwrap().push((locale, gate.clone()));

		async move {
			gate.wait().await;
			Ok::<_, String>(locale.as_bytes().to_vec())
		}
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	let label = app
		.world_mut()
		.spawn(LocalizedText::<TestCatalog>::new(|catalog| {
			catalog.0.clone()
		}))
		.id();
	(app, attempts, label)
}

fn count(attempts: &Attempts, locale: &str) -> usize {
	attempts
		.lock()
		.unwrap()
		.iter()
		.filter(|(language, _)| *language == locale)
		.count()
}

fn release(attempts: &Attempts, locale: &str, index: usize) {
	attempts
		.lock()
		.unwrap()
		.iter()
		.filter(|(language, _)| *language == locale)
		.nth(index)
		.unwrap()
		.1
		.release();
}

#[test]
fn setter_keeps_current_resource_and_text_then_commits_without_extra_calls() {
	let (mut app, attempts, label) = app();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |_| count(&attempts, "es") == 1);

	for _ in 0..5 {
		app.update();
		assert_eq!(
			app.world().resource::<Localization<TestCatalog>>().locale(),
			"ja"
		);
		assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
		assert_eq!(app.world().get::<Text>(label).unwrap().0, "ja");
	}

	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	app.update();
	assert_eq!(count(&attempts, "es"), 1);
	release(&attempts, "es", 0);
	pump(&mut app, |world| {
		world.resource::<Localization<TestCatalog>>().locale() == "es"
	});
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "es");
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.prepared_locale(),
		None
	);
}

#[test]
fn latest_target_wins_and_selecting_active_locale_cancels() {
	let (mut app, attempts, label) = app();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |_| count(&attempts, "es") == 1);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("ja");
	release(&attempts, "es", 0);
	app.update();
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "ja");
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.prepared_locale(),
		None
	);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |_| count(&attempts, "es") == 2);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("de");
	pump(&mut app, |_| count(&attempts, "de") == 1);
	release(&attempts, "es", 1);
	app.update();
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().locale(),
		"ja"
	);
	release(&attempts, "de", 0);
	pump(&mut app, |world| {
		world.resource::<Localization<TestCatalog>>().locale() == "de"
	});
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "de");
}

#[test]
fn manual_preparation_takes_over_without_automatic_commit() {
	let (mut app, attempts, _) = app();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |_| count(&attempts, "es") == 1);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.prepare_locale("es");
	release(&attempts, "es", 0);
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog>>()
			.preparation_status()
			== PreparationStatus::Ready
	});
	app.update();
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().locale(),
		"ja"
	);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().locale(),
		"es"
	);
}

#[test]
fn initial_setter_does_not_read_the_abandoned_default_language() {
	let (mut app, attempts, _) = app();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |_| count(&attempts, "es") == 1);
	assert_eq!(count(&attempts, "ja"), 0);
	release(&attempts, "es", 0);
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().locale(),
		"es"
	);
}

#[test]
fn manual_pin_and_automatic_owners_are_independent() {
	let (mut app, _, label) = app();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	{
		let mut localization = app.world_mut().resource_mut::<Localization<TestCatalog>>();
		localization.load::<TestCatalog>();
		localization.load::<TestCatalog>();
		localization.unload::<TestCatalog>();
	}
	app.update();
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "ja");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.load::<TestCatalog>();
	app.world_mut().despawn(label);
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.unload::<TestCatalog>();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn failed_target_is_observable_retains_text_and_manual_load_retries_it() {
	let attempts = Arc::new(AtomicUsize::new(0));
	let observed = attempts.clone();
	let plugin = LocalizationPlugin::<TestCatalog>::from_loader(move |locale, _| {
		std::future::ready(
			if locale == "es" && observed.fetch_add(1, Ordering::Relaxed) == 0 {
				Err("target unavailable".to_owned())
			} else {
				Ok(locale.as_bytes().to_vec())
			},
		)
	});
	let errors = Arc::new(Mutex::new(Vec::new()));
	let reported = errors.clone();
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin)).add_systems(
		Update,
		move |mut updates: CatalogUpdateReader<TestCatalog>| {
			for update in updates.read() {
				if let CatalogUpdate::Rejected { locale, .. } = update {
					reported.lock().unwrap().push(*locale);
				}
			}
		},
	);
	let label = app
		.world_mut()
		.spawn(LocalizedText::<TestCatalog>::new(|catalog| {
			catalog.0.clone()
		}))
		.id();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog>>()
				.preparation_status(),
			PreparationStatus::Failed(_)
		)
	});
	app.update();
	let changed = app
		.world()
		.get_resource_ref::<Localization<TestCatalog>>()
		.unwrap()
		.last_changed();

	for _ in 0..5 {
		app.update();
		assert_eq!(
			app.world()
				.get_resource_ref::<Localization<TestCatalog>>()
				.unwrap()
				.last_changed(),
			changed
		);
	}

	assert_eq!(attempts.load(Ordering::Relaxed), 1);
	assert_eq!(*errors.lock().unwrap(), [Some("es")]);
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "ja");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world.resource::<Localization<TestCatalog>>().locale() == "es"
	});
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "es");
}

#[test]
fn repeating_initial_selection_retries_failed_active_leaves_without_a_manual_pin() {
	let attempts = Arc::new(AtomicUsize::new(0));
	let observed = attempts.clone();
	let plugin = LocalizationPlugin::<TestCatalog>::from_loader(move |locale, _| {
		assert_eq!(locale, "es");
		std::future::ready(if observed.fetch_add(1, Ordering::Relaxed) == 0 {
			Err("initial load failed".to_owned())
		} else {
			Ok(locale.as_bytes().to_vec())
		})
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	let binding = app
		.world_mut()
		.spawn(LocalizedText::<TestCatalog>::new(|catalog| {
			catalog.0.clone()
		}))
		.id();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog>>()
				.status::<TestCatalog>(),
			crate::ModuleStatus::Failed(_)
		)
	});
	assert_eq!(attempts.load(Ordering::Relaxed), 1);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(app.world().get::<Text>(binding).unwrap().0, "es");
	assert_eq!(attempts.load(Ordering::Relaxed), 2);
	app.world_mut().despawn(binding);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn reload_during_switch_retries_target_and_still_commits_automatically() {
	let (mut app, attempts, label) = app();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |_| count(&attempts, "es") == 1);
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(crate::ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
	app.update();
	release(&attempts, "es", 0);
	pump(&mut app, |_| count(&attempts, "es") == 2);
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().locale(),
		"ja"
	);
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "ja");
	release(&attempts, "es", 1);
	pump(&mut app, |world| {
		world.resource::<Localization<TestCatalog>>().locale() == "es"
	});
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "es");
}
