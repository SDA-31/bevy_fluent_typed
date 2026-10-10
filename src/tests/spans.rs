use super::TestCatalog;
use crate::bevy::prelude::*;
use crate::{Lazy, Localization, LocalizationPlugin, LocalizedText};
use std::{
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	time::{Duration, Instant},
};

fn pump(app: &mut App, value: Option<&str>, spans: &[Entity]) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if spans
			.iter()
			.all(|entity| app.world().get::<TextSpan>(*entity).unwrap().0 == value.unwrap_or(""))
		{
			return;
		}

		assert!(Instant::now() < deadline, "span refresh timed out");
		std::thread::sleep(Duration::from_millis(1));
	}
}

#[test]
fn ui_and_world_spans_follow_catalog_lifetime_without_becoming_roots() {
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let binding = LocalizedText::<TestCatalog>::new(move |catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	});

	let plugin = LocalizationPlugin::<TestCatalog, Lazy>::from_bytes([
		("ja", "ui.ftl", b"Japanese".as_slice()),
		("es", "ui.ftl", b"Spanish".as_slice()),
	])
	.unwrap();
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	let ui = app.world_mut().spawn(Text("parent".into())).id();
	let world = app.world_mut().spawn(Text2d("parent".into())).id();
	let spans: Vec<_> = [ui, world]
		.into_iter()
		.map(|parent| {
			app.world_mut()
				.spawn((TextSpan("stale".into()), ChildOf(parent), binding.clone()))
				.id()
		})
		.collect();
	app.finish();
	app.cleanup();

	pump(&mut app, None, &spans);
	assert_eq!(calls.load(Ordering::Relaxed), 0);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, Some("Japanese"), &spans);
	let before = calls.load(Ordering::Relaxed);
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), before);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.set_locale("es");
	pump(&mut app, Some("Spanish"), &spans);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.unload::<TestCatalog>();
	pump(&mut app, None, &spans);

	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Lazy>>()
		.load::<TestCatalog>();
	pump(&mut app, Some("Spanish"), &spans);

	for entity in &spans {
		assert!(app.world().get::<Text>(*entity).is_none());
		assert!(app.world().get::<Text2d>(*entity).is_none());
	}

	assert_eq!(app.world().get::<Text>(ui).unwrap().0, "parent");
	assert_eq!(app.world().get::<Text2d>(world).unwrap().0, "parent");

	app.world_mut()
		.entity_mut(spans[0])
		.insert(LocalizedText::<TestCatalog>::new(|catalog| {
			format!("new {}", catalog.0)
		}));
	pump(&mut app, Some("new Spanish"), &spans[..1]);
	assert_eq!(app.world().get::<TextSpan>(spans[1]).unwrap().0, "Spanish");

	for entity in spans {
		app.world_mut().despawn(entity);
	}

	app.update();
	let later = app.world_mut().spawn((TextSpan::default(), binding)).id();
	pump(&mut app, Some("Spanish"), &[later]);
	assert!(app.world().get::<Text>(later).is_none());
}

#[test]
fn automatic_spans_and_manual_pins_retain_and_release_independently() {
	let plugin =
		LocalizationPlugin::<TestCatalog>::from_bytes([("ja", "ui.ftl", b"Japanese".as_slice())])
			.unwrap();
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	let binding = LocalizedText::<TestCatalog>::new(|catalog| catalog.0.clone());
	let root = app
		.world_mut()
		.spawn((Text::default(), binding.clone()))
		.id();
	let span = app
		.world_mut()
		.spawn((TextSpan::default(), ChildOf(root), binding.clone()))
		.id();
	pump(&mut app, Some("Japanese"), &[span]);
	app.world_mut()
		.entity_mut(root)
		.remove::<LocalizedText<TestCatalog>>();
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.load::<TestCatalog>();
	app.world_mut().despawn(span);
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.unload::<TestCatalog>();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	let later = app.world_mut().spawn((TextSpan::default(), binding)).id();
	pump(&mut app, Some("Japanese"), &[later]);
	assert!(app.world().contains_resource::<TestCatalog>());
	assert!(app.world().get::<Text>(later).is_none());
	assert!(app.world().get::<Text2d>(later).is_none());
	app.world_mut().despawn(later);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}
