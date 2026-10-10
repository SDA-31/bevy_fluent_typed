//! Independent typed progress views share registration without creating demand.
use super::{Hud, Other, Panel, Presentation, Root, Singleton, configured, pump};
use crate::bevy::prelude::*;
use crate::{
	FluentScope, Lazy, Localization, LocalizationPlugin, LocalizationProgress,
	LocalizationProgressPlugin, PreparationStatus,
};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[test]
fn recursive_root_plugin_shares_schedules_and_does_not_load_unrequested_files() {
	use Hud as Interface;

	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins(LocalizationProgressPlugin::<Root>::default());
	assert!(!app.world().contains_resource::<LocalizationProgress<Hud>>());

	app.add_plugins(LocalizationPlugin::<Root, Lazy>::from_loader(
		move |locale, path| {
			observed.fetch_add(1, Ordering::Relaxed);
			std::future::ready(Ok::<_, String>(format!("{locale}:{path}").into_bytes()))
		},
	));

	let pre = app
		.world()
		.resource::<Schedules>()
		.get(PreUpdate)
		.unwrap()
		.systems_len();
	let post = app
		.world()
		.resource::<Schedules>()
		.get(PostUpdate)
		.unwrap()
		.systems_len();
	let hud_tick = app
		.world()
		.get_resource_ref::<LocalizationProgress<Hud>>()
		.unwrap()
		.last_changed();

	app.add_plugins(LocalizationProgressPlugin::<Panel>::default());
	app.add_plugins(LocalizationProgressPlugin::<Interface>::default());

	assert_eq!(
		app.world()
			.resource::<Schedules>()
			.get(PreUpdate)
			.unwrap()
			.systems_len(),
		pre
	);
	assert_eq!(
		app.world()
			.resource::<Schedules>()
			.get(PostUpdate)
			.unwrap()
			.systems_len(),
		post
	);
	assert_eq!(
		app.world()
			.get_resource_ref::<LocalizationProgress<Hud>>()
			.unwrap()
			.last_changed(),
		hud_tick
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Root>>()
			.active()
			.total,
		0
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Presentation>>()
			.active()
			.total,
		2
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.active()
			.unloaded,
		1
	);

	app.finish();
	app.cleanup();
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), 0);
	assert!(!app.world().contains_resource::<Hud>());

	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.load::<Hud>();
	pump(&mut app, |world| {
		world.resource::<LocalizationProgress<Hud>>().active().ready == 1
	});
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Root>>()
			.active()
			.total,
		1
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Presentation>>()
			.active()
			.unloaded,
		1
	);

	let tick = app
		.world()
		.get_resource_ref::<LocalizationProgress<Hud>>()
		.unwrap()
		.last_changed();

	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.load::<Other>();
	pump(&mut app, |world| world.contains_resource::<Other>());
	assert_eq!(
		app.world()
			.get_resource_ref::<LocalizationProgress<Hud>>()
			.unwrap()
			.last_changed(),
		tick
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Root>>()
			.active()
			.ready,
		2
	);

	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		world
			.resource::<LocalizationProgress<Hud>>()
			.preparation_status()
			== &PreparationStatus::Ready
	});
	let group = app.world().resource::<LocalizationProgress<Presentation>>();
	assert_eq!(group.preparation().unwrap().ready, 1);
	assert_eq!(group.preparation().unwrap().unloaded, 1);
	assert_eq!(group.preparation_status(), &PreparationStatus::Ready);

	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.commit_locale()
		.unwrap();
	app.update();
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.active()
			.locale,
		"es"
	);
	assert!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.preparation()
			.is_none()
	);
	assert_eq!(calls.load(Ordering::Relaxed), 4);

	let weak = Arc::downgrade(&app.world().resource::<Hud>().0);

	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.unload::<Hud>();
	app.update();
	assert!(weak.upgrade().is_none());
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.active()
			.unloaded,
		1
	);
}

#[test]
fn ready_scope_counters_cannot_hide_another_requested_modules_preparation_failure() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<Root>::from_bytes([
			("en", Hud::module_paths()[0], b"hud".as_slice()),
			("en", Panel::module_paths()[0], b"panel".as_slice()),
			("en", Other::module_paths()[0], b"other".as_slice()),
			("es", Hud::module_paths()[0], b"hud".as_slice()),
			("es", Panel::module_paths()[0], b"panel".as_slice()),
		])
		.unwrap(),
		LocalizationProgressPlugin::<Hud>::default(),
		LocalizationProgressPlugin::<Presentation>::default(),
	));

	app.finish();
	app.cleanup();
	pump(&mut app, |world| world.contains_resource::<Root>());
	assert!(
		!app.world()
			.contains_resource::<LocalizationProgress<Root>>()
	);

	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.prepare_locale("es");
	pump(&mut app, |world| {
		let hud = world.resource::<LocalizationProgress<Hud>>();
		let group = world.resource::<LocalizationProgress<Presentation>>();
		hud.preparation().is_some_and(|value| value.ready == 1)
			&& group.preparation().is_some_and(|value| value.ready == 2)
			&& matches!(hud.preparation_status(), PreparationStatus::Failed(_))
	});
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.active()
			.locale,
		"en"
	);
	assert!(
		app.world_mut()
			.resource_mut::<Localization<Root>>()
			.commit_locale()
			.is_err()
	);

	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.cancel_preparation();
	app.update();
	assert!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.preparation()
			.is_none()
	);

	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.prepare_locale("en");
	app.update();
	let hud = app.world().resource::<LocalizationProgress<Hud>>();
	assert_eq!(hud.active(), hud.preparation().unwrap());
	assert_eq!(hud.preparation_status(), &PreparationStatus::Ready);
}

#[test]
fn scope_subscriptions_bind_to_their_own_provider_and_loading_mode() {
	use crate::tests::TestCatalog;

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationProgressPlugin::<Hud>::default(),
		LocalizationProgressPlugin::<TestCatalog>::default(),
		LocalizationPlugin::<Root, Lazy>::from_bytes([(
			"en",
			Hud::module_paths()[0],
			b"hud".as_slice(),
		)])
		.unwrap(),
		LocalizationPlugin::<TestCatalog>::from_bytes([("ja", "ui.ftl", b"ja".as_slice())])
			.unwrap(),
	));

	app.finish();
	app.cleanup();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert!(!app.world().contains_resource::<Hud>());
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<TestCatalog>>()
			.active()
			.locale,
		"ja"
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<TestCatalog>>()
			.active()
			.ready,
		1
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.active()
			.locale,
		"en"
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.active()
			.unloaded,
		1
	);

	let tick = app
		.world()
		.get_resource_ref::<LocalizationProgress<TestCatalog>>()
		.unwrap()
		.last_changed();

	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.load::<Hud>();
	pump(&mut app, |world| world.contains_resource::<Hud>());
	assert_eq!(
		app.world()
			.get_resource_ref::<LocalizationProgress<TestCatalog>>()
			.unwrap()
			.last_changed(),
		tick
	);
	assert_eq!(
		app.world()
			.resource::<LocalizationProgress<Hud>>()
			.active()
			.ready,
		1
	);
}

#[test]
fn group_recursion_excludes_ancestors_and_siblings_even_with_identical_leaf_sets() {
	let (mut app, source) = configured();
	app.add_plugins(LocalizationProgressPlugin::<Singleton>::default());
	assert!(
		app.world()
			.contains_resource::<LocalizationProgress<Singleton>>()
	);
	assert!(app.world().contains_resource::<LocalizationProgress<Hud>>());
	assert!(
		!app.world()
			.contains_resource::<LocalizationProgress<Presentation>>()
	);
	assert!(
		!app.world()
			.contains_resource::<LocalizationProgress<Panel>>()
	);
	assert!(
		!app.world()
			.contains_resource::<LocalizationProgress<Other>>()
	);
	assert!(
		!app.world()
			.contains_resource::<LocalizationProgress<Root>>()
	);
	let tick = app
		.world()
		.get_resource_ref::<LocalizationProgress<Hud>>()
		.unwrap()
		.last_changed();

	app.add_plugins(LocalizationProgressPlugin::<Presentation>::default());
	assert!(
		app.world()
			.contains_resource::<LocalizationProgress<Panel>>()
	);
	assert!(
		!app.world()
			.contains_resource::<LocalizationProgress<Other>>()
	);
	assert!(
		!app.world()
			.contains_resource::<LocalizationProgress<Root>>()
	);
	assert_eq!(
		app.world()
			.get_resource_ref::<LocalizationProgress<Hud>>()
			.unwrap()
			.last_changed(),
		tick
	);
	app.finish();
	app.cleanup();
	app.update();
	assert_eq!(source.count(), 0);
}
