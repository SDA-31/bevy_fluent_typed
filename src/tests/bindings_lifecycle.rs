use super::{TestCatalog, manifest};
use crate::bevy::prelude::*;
use crate::{Localization, LocalizationPlugin, LocalizedText, Manual};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

fn binding(calls: &Arc<AtomicUsize>) -> LocalizedText<TestCatalog> {
	let calls = calls.clone();
	LocalizedText::new(move |catalog: &TestCatalog| {
		calls.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	})
}

#[test]
fn bindings_track_lifetimes_before_and_after_plugin_installation() {
	let calls = Arc::new(AtomicUsize::new(0));
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()));
	let cancelled = app.world_mut().spawn(binding(&calls)).id();
	app.world_mut().despawn(cancelled);
	let first = app
		.world_mut()
		.spawn((Text::default(), binding(&calls)))
		.id();
	let second = app
		.world_mut()
		.spawn((Text2d::default(), binding(&calls)))
		.id();
	app.add_plugins(LocalizationPlugin::<TestCatalog>::new_manual(manifest()));
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), 2);
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), 2);

	app.world_mut().despawn(first);
	app.world_mut().entity_mut(second).insert(binding(&calls));
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), 3);
	*app.world_mut()
		.get_mut::<LocalizedText<TestCatalog>>(second)
		.unwrap() = binding(&calls);
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), 4);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	app.update();
	assert_eq!(app.world().get::<Text2d>(second).unwrap().0, "");
	assert_eq!(calls.load(Ordering::Relaxed), 4);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	app.update();
	assert_eq!(app.world().get::<Text2d>(second).unwrap().0, "ja");
	assert_eq!(calls.load(Ordering::Relaxed), 5);

	app.world_mut().despawn(second);
	app.update();
	let later = app
		.world_mut()
		.spawn((Text::default(), binding(&calls)))
		.id();
	app.update();
	assert_eq!(app.world().get::<Text>(later).unwrap().0, "ja");
	assert_eq!(calls.load(Ordering::Relaxed), 6);
}

#[test]
fn missing_default_text_is_restored_in_the_same_frame() {
	let calls = Arc::new(AtomicUsize::new(0));
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<TestCatalog>::new(manifest()),
	));
	let entity = app.world_mut().spawn(binding(&calls)).id();
	app.update();
	app.update();
	assert_eq!(app.world().get::<Text>(entity).unwrap().0, "ja");
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	app.world_mut().entity_mut(entity).remove::<Text>();
	app.update();
	assert_eq!(app.world().get::<Text>(entity).unwrap().0, "ja");
	assert_eq!(calls.load(Ordering::Relaxed), 2);
}
