//! Headless driver: ordinary applications use Bevy's App::run().
use crate::{
	texts::{self, ui::Greeting},
	widgets::{
		Widgets, pilot,
		targets::{Badge, Caption, CaptionMirror, CaptionPaused, Title},
	},
};
use bevy_fluent_typed::bevy::{
	a11y::AccessibilityNode,
	prelude::{AccessibleLabel, App, Color, Text, Text2d, TextColor, TextSpan, World},
};
use bevy_fluent_typed::{CachePolicy, Localization, Localized, ModuleStatus};
use std::time::{Duration, Instant};

const LOAD_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(1);

pub(super) fn cache_policy() -> Result<CachePolicy, String> {
	let arguments: Vec<_> = std::env::args().skip(1).collect();

	match arguments.as_slice() {
		[] => Ok(CachePolicy::Rendered),
		[argument] if argument == "--no-cache" => Ok(CachePolicy::None),
		_ => Err("usage: localization-bindings-example [--no-cache]".into()),
	}
}

pub(super) fn run(mut app: App) -> Result<(), String> {
	app.finish();
	app.cleanup();

	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		wait(&mut app, |world| {
			world
				.get_resource::<Greeting>()
				.is_some_and(|catalog| catalog.locale() == locale)
		})?;
		verify_native_targets(app.world());
		print!("{locale}: ");
		report(app.world());
	}

	let widgets = app.world().resource::<Widgets>();
	let caption = widgets.caption;
	let title = widgets.title;
	let badge = widgets.badge;
	let native = [widgets.ui, widgets.world, widgets.button, widgets.badge];
	let original = app.world().get::<Text>(widgets.ui).unwrap().0.clone();
	let message = pilot("Grace");
	app.world_mut()
		.entity_mut(caption)
		.insert(Localized::<Caption, _>::from(message.clone()));
	app.world_mut()
		.entity_mut(badge)
		.insert(Localized::<Badge, _>::from(message));
	app.update();
	assert_eq!(app.world().get::<Text>(native[0]).unwrap().0, original);
	print!("updated: ");
	report(app.world());

	// A structural filter pauses writes; reentry catches up to a newer Msg.
	let saved = app.world().get::<Caption>(caption).unwrap().text.clone();
	app.world_mut()
		.entity_mut(caption)
		.insert((CaptionPaused, Localized::<Caption, _>::from(pilot("Lin"))));
	app.update();
	assert_eq!(app.world().get::<Caption>(caption).unwrap().text, saved);
	app.world_mut()
		.entity_mut(caption)
		.remove::<CaptionPaused>();
	app.update();
	assert_eq!(
		app.world().get::<Caption>(caption).unwrap().text,
		app.world().resource::<Greeting>().msg_detail("Lin"),
	);

	// Only Title remains as a consumer after these independent removals.
	for entity in native {
		app.world_mut().despawn(entity);
	}

	app.world_mut()
		.entity_mut(caption)
		.remove::<Localized<Caption, Greeting>>();
	app.update();
	assert!(app.world().get::<Caption>(caption).is_some());
	println!(
		"remaining: {}",
		app.world().resource::<Greeting>().msg_title()
	);
	app.world_mut().despawn(title);
	wait(&mut app, |world| !world.contains_resource::<Greeting>())?;
	println!("unloaded");
	Ok(())
}

fn verify_native_targets(world: &World) {
	let widgets = world.resource::<Widgets>();
	let catalog = world.resource::<Greeting>();
	let expected = catalog.msg_detail("Ada");
	assert_eq!(world.get::<Text>(widgets.ui).unwrap().0, expected);
	assert_eq!(world.get::<Text2d>(widgets.world).unwrap().0, expected);
	assert_eq!(world.get::<TextSpan>(widgets.span).unwrap().0, expected);
	assert_eq!(world.get::<Text>(widgets.button_text).unwrap().0, expected);
	assert_eq!(world.get::<Badge>(widgets.badge).unwrap().text, expected);
	assert_eq!(
		world.get::<AccessibleLabel>(widgets.button).unwrap().0,
		expected
	);
	assert_eq!(
		world
			.get::<AccessibilityNode>(widgets.button)
			.unwrap()
			.label(),
		Some(expected.as_str())
	);
	assert_eq!(
		world.get::<TextColor>(widgets.ui).unwrap().0,
		Color::srgb(0.2, 0.8, 0.9)
	);
	assert!(world.get::<AccessibleLabel>(widgets.ui).is_none());
	assert!(world.get::<Text>(widgets.world).is_none());
	assert!(world.get::<Text>(widgets.span).is_none());
	assert!(world.get::<Text2d>(widgets.span).is_none());
	assert_eq!(
		world.get::<Caption>(widgets.caption).unwrap().text,
		expected
	);
	assert_eq!(
		world.get::<CaptionMirror>(widgets.caption).unwrap().0,
		expected
	);
}

fn report(world: &World) {
	let widgets = world.resource::<Widgets>();
	let caption = world.get::<Caption>(widgets.caption).unwrap();
	let title = world.get::<Title>(widgets.title).unwrap();
	let badge = world.get::<Badge>(widgets.badge).unwrap();
	println!("{} | {} | {}", caption.text, title.text, badge.text);
}

fn wait(app: &mut App, ready: impl Fn(&World) -> bool) -> Result<(), String> {
	let deadline = Instant::now() + LOAD_TIMEOUT;

	loop {
		app.update();

		if let ModuleStatus::Failed(error) = app
			.world()
			.resource::<Localization<texts::Translations>>()
			.status::<Greeting>()
		{
			return Err(error.to_string());
		}

		if ready(app.world()) {
			return Ok(());
		}

		if Instant::now() >= deadline {
			return Err("binding example timed out".into());
		}

		std::thread::sleep(POLL_INTERVAL);
	}
}
