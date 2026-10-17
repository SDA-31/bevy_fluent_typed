use super::{Hud, Other, Presentation, Root, pump};
use crate::bevy::prelude::{App, MinimalPlugins, Text};
use crate::cache_policy::for_scope;
use crate::tests::TestCatalog;
use crate::{CachePolicy, LocalizationPlugin, Localized};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[test]
fn cache_policy_covers_child_scopes_and_keeps_other_providers_independent() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<Root>::from_loader(|_, path| {
			std::future::ready(Ok::<_, String>(path.as_bytes().to_vec()))
		})
		.with_cache_policy(CachePolicy::None),
		LocalizationPlugin::<TestCatalog>::from_bytes([(
			"ja",
			"ui.ftl",
			b"independent".as_slice(),
		)])
		.unwrap(),
	));
	assert!(matches!(for_scope::<Root>(app.world()), CachePolicy::None));
	assert!(matches!(
		for_scope::<Presentation>(app.world()),
		CachePolicy::None
	));
	assert!(matches!(for_scope::<Hud>(app.world()), CachePolicy::None));
	assert!(matches!(for_scope::<Other>(app.world()), CachePolicy::None));
	assert!(matches!(
		for_scope::<TestCatalog>(app.world()),
		CachePolicy::Rendered
	));
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let child = app
		.world_mut()
		.spawn(Localized::<Text, Hud>::new(move |hud| {
			observed.fetch_add(1, Ordering::Relaxed);
			hud.0.as_str().to_owned()
		}))
		.id();
	let observed = calls.clone();
	let other = app
		.world_mut()
		.spawn(Localized::<Text, TestCatalog>::new(move |catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		}))
		.id();
	pump(&mut app, |world| {
		world
			.get::<Text>(child)
			.is_some_and(|text| text.0 == "presentation/hud.ftl")
			&& world
				.get::<Text>(other)
				.is_some_and(|text| text.0 == "independent")
	});
	let initial = calls.load(Ordering::Relaxed);
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), initial);
	app.world_mut().get_mut::<Text>(child).unwrap().0.clear();
	app.world_mut().get_mut::<Text>(other).unwrap().0.clear();
	app.update();
	assert_eq!(
		calls.load(Ordering::Relaxed),
		initial + 1,
		"only the uncached provider formats again for repair"
	);
	assert_eq!(
		app.world().get::<Text>(child).unwrap().0,
		"presentation/hud.ftl"
	);
	assert_eq!(app.world().get::<Text>(other).unwrap().0, "independent");
}

#[test]
fn uncached_plugin_adopts_bindings_inserted_before_configuration() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let owner = app
		.world_mut()
		.spawn(Localized::<Text, TestCatalog>::new(move |catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		}))
		.id();
	app.world_mut().flush();
	app.add_plugins(
		LocalizationPlugin::<TestCatalog>::from_bytes([("ja", "ui.ftl", b"ready".as_slice())])
			.unwrap()
			.with_cache_policy(CachePolicy::None),
	);
	pump(&mut app, |world| {
		world
			.get::<Text>(owner)
			.is_some_and(|text| text.0 == "ready")
	});
	let initial = calls.load(Ordering::Relaxed);
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), initial);
	app.world_mut().get_mut::<Text>(owner).unwrap().0.clear();
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), initial + 1);
	assert_eq!(app.world().get::<Text>(owner).unwrap().0, "ready");
}
