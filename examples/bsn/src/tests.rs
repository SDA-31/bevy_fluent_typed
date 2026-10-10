use crate::{greeting_scene, texts};
use bevy::{asset::AssetPlugin, prelude::*, scene::ScenePlugin};
use bevy_fluent_typed::{
	Localization, LocalizationPlugin, LocalizedText, Message, ModuleStatus, ReloadCatalogs,
};
use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
	time::{Duration, Instant},
};
use texts::presentation::Hud as Interface;

#[path = "tests/native/bevy_0_20.rs"]
mod native;

type Source = Arc<Mutex<HashMap<String, Vec<u8>>>>;

fn test_app() -> (App, Source) {
	let source: Source = Arc::new(Mutex::new(HashMap::from([
		("en".into(), b"hello = Hello, { $name }!".to_vec()),
		("es".into(), "hello = Hola, { $name }!".as_bytes().to_vec()),
		(
			"ru".into(),
			"hello = Привет, { $name }!".as_bytes().to_vec(),
		),
	])));
	let loader_source = source.clone();
	let plugin = LocalizationPlugin::<texts::Translations>::from_loader(move |locale, path| {
		assert_eq!(path, Interface::PATH);
		let bytes = loader_source
			.lock()
			.unwrap()
			.get(locale.as_ref())
			.cloned()
			.ok_or_else(|| "missing test locale".to_string());
		std::future::ready(bytes)
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin, plugin));
	app.finish();
	app.cleanup();
	(app, source)
}

fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if ready(app.world()) {
			return;
		}

		assert!(Instant::now() < deadline, "BSN localization timed out");
		std::thread::sleep(Duration::from_millis(1));
	}
}

fn reload(app: &mut App) {
	app.world_mut()
		.write_message(ReloadCatalogs::<texts::Translations>::default());
}

fn pair_ready(world: &World, pair: (Entity, Entity), prefix: &str) -> bool {
	world
		.get::<Text>(pair.0)
		.is_some_and(|text| text.0.starts_with(prefix))
		&& world
			.get::<Text2d>(pair.1)
			.is_some_and(|text| text.0.starts_with(prefix))
}

#[test]
fn native_bsn_bindings_request_switch_locale_and_release_after_last_removal() {
	let (mut app, _) = test_app();
	let name = String::from("Ada");
	let binding = LocalizedText::<Interface>::new(move |hud| hud.msg_hello(&name));
	let pair = native::spawn(app.world_mut(), binding);
	assert!(!app.world().contains_resource::<Interface>());
	assert!(
		app.world()
			.get::<Text>(pair.0)
			.is_none_or(|text| text.0.is_empty())
	);
	assert!(app.world().get::<Text2d>(pair.1).unwrap().0.is_empty());
	assert!(app.world().get::<Text>(pair.1).is_none());

	for (locale, prefix) in [
		(texts::Locale::En, "Hello"),
		(texts::Locale::Es, "Hola"),
		(texts::Locale::Ru, "Привет"),
	] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		pump(&mut app, |world| pair_ready(world, pair, prefix));
		assert!(app.world().get::<Text>(pair.0).unwrap().0.contains("Ada"));
		assert_eq!(
			app.world().get::<Text>(pair.0).unwrap().0,
			app.world().get::<Text2d>(pair.1).unwrap().0
		);
	}

	app.world_mut()
		.entity_mut(pair.0)
		.remove::<LocalizedText<Interface>>();
	app.update();
	assert!(app.world().contains_resource::<Interface>());
	assert!(
		app.world()
			.get::<Text2d>(pair.1)
			.unwrap()
			.0
			.starts_with("Привет")
	);
	app.world_mut()
		.entity_mut(pair.1)
		.remove::<LocalizedText<Interface>>();
	app.update();
	assert!(!app.world().contains_resource::<Interface>());
	app.world_mut()
		.entity_mut(pair.0)
		.insert(LocalizedText::<Interface>::new(|hud| {
			hud.msg_hello("Grace")
		}));
	pump(&mut app, |world| {
		world
			.get::<Text>(pair.0)
			.is_some_and(|text| text.0.starts_with("Привет") && text.0.contains("Grace"))
	});
}

#[test]
fn native_bsn_reload_preserves_last_good_text_and_recovers() {
	let (mut app, source) = test_app();
	let pair = native::spawn(
		app.world_mut(),
		LocalizedText::<Interface>::new(|hud| hud.msg_hello("Lin")),
	);
	pump(&mut app, |world| pair_ready(world, pair, "Hello"));
	source
		.lock()
		.unwrap()
		.insert("en".into(), b"hello = Updated, { $name }!".to_vec());
	reload(&mut app);
	pump(&mut app, |world| pair_ready(world, pair, "Updated"));
	let good = app.world().get::<Text>(pair.0).unwrap().0.clone();
	source
		.lock()
		.unwrap()
		.insert("en".into(), b"hello = Broken contract".to_vec());
	reload(&mut app);
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<texts::Translations>>()
				.status::<Interface>(),
			ModuleStatus::Failed(_)
		)
	});
	assert_eq!(app.world().get::<Text>(pair.0).unwrap().0, good);
	assert_eq!(app.world().get::<Text2d>(pair.1).unwrap().0, good);
	source
		.lock()
		.unwrap()
		.insert("en".into(), b"hello = Recovered, { $name }!".to_vec());
	reload(&mut app);
	pump(&mut app, |world| pair_ready(world, pair, "Recovered"));
}

#[test]
fn common_scene_factory_keeps_each_instances_owned_arguments() {
	let (mut app, _) = test_app();
	let first = app
		.world_mut()
		.spawn_scene(greeting_scene("Ada".into()))
		.unwrap()
		.id();
	let second = app
		.world_mut()
		.spawn_scene(greeting_scene("Lin".into()))
		.unwrap()
		.id();
	pump(&mut app, |world| {
		world
			.get::<Text>(first)
			.is_some_and(|text| text.0.starts_with("Hello"))
			&& world
				.get::<Text>(second)
				.is_some_and(|text| text.0.starts_with("Hello"))
	});
	assert!(app.world().get::<Text>(first).unwrap().0.contains("Ada"));
	assert!(app.world().get::<Text>(second).unwrap().0.contains("Lin"));
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Es);
	pump(&mut app, |world| {
		world
			.get::<Text>(first)
			.is_some_and(|text| text.0.starts_with("Hola"))
			&& world
				.get::<Text>(second)
				.is_some_and(|text| text.0.starts_with("Hola"))
	});
	assert!(app.world().get::<Text>(first).unwrap().0.contains("Ada"));
	assert!(app.world().get::<Text>(second).unwrap().0.contains("Lin"));
}

#[test]
fn inline_from_constructors_preserve_message_and_component_arguments() {
	let (mut app, _) = test_app();
	let name = String::from("Ada");
	let message = Message::<Interface>::new(move |hud| hud.msg_hello(&name));
	let binding = LocalizedText::from(message.clone());
	let label = app
		.world_mut()
		.spawn_scene(bsn! {
			Text2d
			LocalizedText::<_>::from(message)
		})
		.unwrap()
		.id();
	let from_binding = app
		.world_mut()
		.spawn_scene(bsn! {
			Text
			LocalizedText::<Interface>::from(binding)
		})
		.unwrap()
		.id();
	pump(&mut app, |world| {
		world
			.get::<Text2d>(label)
			.is_some_and(|text| text.0.contains("Ada"))
			&& world
				.get::<Text>(from_binding)
				.is_some_and(|text| text.0.contains("Ada"))
	});
	assert!(app.world().get::<Text>(label).is_none());
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Es);
	pump(&mut app, |world| {
		world
			.get::<Text2d>(label)
			.is_some_and(|text| text.0.starts_with("Hola"))
			&& world
				.get::<Text>(from_binding)
				.is_some_and(|text| text.0.starts_with("Hola"))
	});
	assert!(app.world().get::<Text2d>(label).unwrap().0.contains("Ada"));
	assert!(
		app.world()
			.get::<Text>(from_binding)
			.unwrap()
			.0
			.contains("Ada")
	);
}

#[test]
fn scene_without_a_message_formatter_returns_an_error() {
	let (mut app, _) = test_app();
	let result = app.world_mut().spawn_scene(bsn! {
		Text
		LocalizedText::<Interface>
	});
	let error = result.err().expect("a message formatter is mandatory");
	assert!(
		error
			.to_string()
			.contains("LocalizedText requires a formatting closure or Message")
	);
}

#[test]
fn native_scene_span_updates_without_inserting_a_root_text() {
	let (mut app, _) = test_app();
	let parent = app.world_mut().spawn(Text::default()).id();
	let span = app
		.world_mut()
		.spawn_scene(bsn! {
			TextSpan
			LocalizedText::<Interface>::new(|hud| hud.msg_hello("Ada"))
		})
		.unwrap()
		.id();
	app.world_mut().entity_mut(parent).add_child(span);
	pump(&mut app, |world| {
		world
			.get::<TextSpan>(span)
			.is_some_and(|text| text.0.starts_with("Hello"))
	});
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Es);
	pump(&mut app, |world| {
		world
			.get::<TextSpan>(span)
			.is_some_and(|text| text.0.starts_with("Hola"))
	});
	assert!(app.world().get::<Text>(span).is_none());
	assert!(app.world().get::<Text2d>(span).is_none());
	app.world_mut().despawn(span);
	app.update();
	assert!(!app.world().contains_resource::<Interface>());
}
