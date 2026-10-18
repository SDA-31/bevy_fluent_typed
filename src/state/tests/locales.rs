//! Multilingual access must preserve selection, ownership and acquisition identity.
use crate::bevy::prelude::{App, MinimalPlugins};
use crate::tests::{TestCatalog, pump};
use crate::{Localization, LocalizationPlugin, Manual, ModuleStatus, Msg, PreparationStatus};
use std::sync::{Arc, Mutex};

type Reads = Arc<Mutex<Vec<&'static str>>>;

fn app() -> (App, Reads) {
	let reads = Reads::default();
	let observed = reads.clone();
	let plugin = LocalizationPlugin::<TestCatalog, Manual>::from_loader(move |locale, _| {
		observed.lock().unwrap().push(locale);
		std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	(app, reads)
}

fn ready(app: &mut App, locale: &'static str) {
	pump(app, |world| {
		world
			.resource::<Localization<TestCatalog, Manual>>()
			.for_locale(locale)
			.catalog()
			.is_some()
	});
}

#[test]
fn inspecting_an_unloaded_locale_does_not_request_it() {
	let (mut app, reads) = app();
	let state = app.world().resource::<Localization<TestCatalog, Manual>>();
	let error = state
		.for_locale("es")
		.modules()
		.get::<TestCatalog>()
		.err()
		.unwrap();
	assert_eq!(error.locale, "es");
	assert_eq!(error.path, "ui.ftl");
	assert_eq!(error.status, ModuleStatus::Unloaded);
	assert_eq!(state.for_locale("es").progress::<TestCatalog>().unloaded, 1);

	for _ in 0..3 {
		app.update();
	}

	assert!(reads.lock().unwrap().is_empty());
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.locale(),
		"ja"
	);
}

#[test]
fn fixed_load_is_readable_through_msg_without_native_resource_publication() {
	let (mut app, reads) = app();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.for_locale_mut("es")
		.load::<TestCatalog>();
	ready(&mut app, "es");
	let state = app.world().resource::<Localization<TestCatalog, Manual>>();
	// Borrow survives dropping the temporary language view.
	let catalog = state
		.for_locale("es")
		.modules()
		.get::<TestCatalog>()
		.unwrap();
	let message = Msg::<TestCatalog>::new(|scope| scope.0.clone());
	assert_eq!(message.render(catalog), "es");
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(state.locale(), "ja");
	assert_eq!(*reads.lock().unwrap(), ["es"]);
}

#[test]
fn switching_to_an_existing_fixed_locale_reuses_acquisition_and_retains_old_owners() {
	let (mut app, reads) = app();
	let lease = {
		let mut state = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Manual>>();
		state.load::<TestCatalog>();
		state.for_locale_mut("es").hold::<TestCatalog>()
	};
	ready(&mut app, "es");
	ready(&mut app, "ja");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.set_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Manual>>()
			.locale() == "es"
	});
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");
	assert_eq!(
		reads
			.lock()
			.unwrap()
			.iter()
			.filter(|&&locale| locale == "es")
			.count(),
		1
	);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.set_locale("de");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Manual>>()
			.locale() == "de"
	});
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.for_locale("es")
			.catalog()
			.is_some()
	);
	drop(lease);
	app.update();
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.for_locale("es")
			.catalog()
			.is_none()
	);
	assert_eq!(app.world().resource::<TestCatalog>().0, "de");
}

#[test]
fn repeated_fixed_pins_are_idempotent_but_leases_have_independent_owners() {
	let (mut app, reads) = app();
	let (first, second) = {
		let mut state = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Manual>>();
		let mut es = state.for_locale_mut("es");
		es.load::<TestCatalog>();
		es.load::<TestCatalog>();
		(es.hold::<TestCatalog>(), es.hold::<TestCatalog>())
	};
	ready(&mut app, "es");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.for_locale_mut("es")
		.unload::<TestCatalog>();
	drop(first);
	app.update();
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.for_locale("es")
			.catalog()
			.is_some()
	);
	assert_eq!(*reads.lock().unwrap(), ["es"]);
	drop(second);
	app.update();
	assert!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.for_locale("es")
			.catalog()
			.is_none()
	);
}

#[test]
fn canceling_preparation_keeps_its_independent_fixed_catalog() {
	let (mut app, reads) = app();
	{
		let mut state = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Manual>>();
		state.load::<TestCatalog>();
		state.for_locale_mut("es").load::<TestCatalog>();
	}
	ready(&mut app, "ja");
	ready(&mut app, "es");
	{
		let mut state = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Manual>>();
		state.prepare_locale("es");
		state.cancel_preparation();
	}
	app.update();
	let state = app.world().resource::<Localization<TestCatalog, Manual>>();
	assert_eq!(state.locale(), "ja");
	assert_eq!(state.preparation_status(), PreparationStatus::Idle);
	assert!(state.for_locale("es").catalog().is_some());
	assert_eq!(
		reads
			.lock()
			.unwrap()
			.iter()
			.filter(|&&locale| locale == "es")
			.count(),
		1
	);
}

#[test]
fn fixed_failure_does_not_block_an_unrelated_language_switch() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<TestCatalog, Manual>::from_loader(|locale, _| {
			std::future::ready(if locale == "de" {
				Err("missing".to_owned())
			} else {
				Ok(locale.as_bytes().to_vec())
			})
		}),
	));
	{
		let mut state = app
			.world_mut()
			.resource_mut::<Localization<TestCatalog, Manual>>();
		state.load::<TestCatalog>();
		state.for_locale_mut("de").load::<TestCatalog>();
	}
	ready(&mut app, "ja");
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.set_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<Localization<TestCatalog, Manual>>()
			.locale() == "es"
	});
	assert_eq!(app.world().resource::<TestCatalog>().0, "es");
	assert!(matches!(
		app.world()
			.resource::<Localization<TestCatalog, Manual>>()
			.for_locale("de")
			.status::<TestCatalog>(),
		ModuleStatus::Failed(_)
	));
}

#[test]
fn reload_includes_requested_fixed_languages_without_loading_unrequested_languages() {
	let (mut app, reads) = app();
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.for_locale_mut("es")
		.load::<TestCatalog>();
	ready(&mut app, "es");
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(crate::ReloadCatalogs::<TestCatalog>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<TestCatalog>::default());
	pump(&mut app, |_| reads.lock().unwrap().len() == 2);
	ready(&mut app, "es");
	assert_eq!(*reads.lock().unwrap(), ["es", "es"]);
}

#[test]
fn native_root_progress_counts_fixed_owners_of_the_active_language_only() {
	let (mut app, _) = app();
	app.add_plugins(crate::LocalizationProgressPlugin::<TestCatalog>::new());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.for_locale_mut("ja")
		.load::<TestCatalog>();
	ready(&mut app, "ja");
	let progress = app
		.world()
		.resource::<crate::LocalizationProgress<TestCatalog>>()
		.active();
	assert_eq!(progress.total, 1);
	assert_eq!(progress.available, 1);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.for_locale_mut("es")
		.load::<TestCatalog>();
	ready(&mut app, "es");
	assert_eq!(
		app.world()
			.resource::<crate::LocalizationProgress<TestCatalog>>()
			.active()
			.total,
		1
	);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.for_locale_mut("ja")
		.unload::<TestCatalog>();
	app.update();
	assert_eq!(
		app.world()
			.resource::<crate::LocalizationProgress<TestCatalog>>()
			.active()
			.total,
		0
	);
}
