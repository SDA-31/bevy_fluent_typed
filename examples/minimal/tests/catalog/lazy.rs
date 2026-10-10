//! Scope demand, navigation lifetimes and independent resource ownership.
use crate::{Locale, Translations, texts};
use localization_runtime::bevy::prelude::*;
use localization_runtime::{
	Full, Lazy, LoadingMode, Localization, LocalizationPlugin, LocalizedText, Message, ModuleStatus,
};

fn app() -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<Translations, Lazy>::new(super::EMBEDDED),
	));
	app.finish();
	app.cleanup();
	app
}

#[test]
fn lazy_plugin_loads_from_a_typed_leaf_only_embedded_manifest() {
	texts::embed_manifest! { const HUD = presentation::Hud; }

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<Translations, Lazy>::new(HUD),
	));
	app.finish();
	app.cleanup();
	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.load::<texts::presentation::Hud>();
	app.update();
	assert_eq!(
		app.world()
			.resource::<texts::presentation::Hud>()
			.msg_title(),
		"Flight HUD"
	);
	assert!(
		!app.world()
			.contains_resource::<texts::presentation::Panel>()
	);
	assert!(!app.world().contains_resource::<Translations>());

	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.set_locale(Locale::Es);
	app.update();
	assert_eq!(
		app.world()
			.resource::<texts::presentation::Hud>()
			.msg_title(),
		"Panel de vuelo"
	);
}

fn assert_idle<M: LoadingMode>(mut app: App) {
	app.add_systems(
		Update,
		|state: Res<Localization<Translations, M>>, mut frames: Local<u32>| {
			if *frames >= 2 {
				assert!(
					!state.is_changed(),
					"idle localization controller must preserve its change tick"
				);
			}

			*frames += 1;
		},
	);

	for _ in 0..5 {
		app.update();
	}
}

#[test]
fn settled_full_empty_lazy_and_partial_lazy_controllers_remain_unchanged() {
	let mut full = App::new();
	full.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<Translations, Full>::new(super::EMBEDDED),
	));
	full.finish();
	full.cleanup();
	assert_idle::<Full>(full);
	assert_idle::<Lazy>(app());
	let mut partial = app();
	partial
		.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.load::<texts::presentation::Hud>();
	assert_idle::<Lazy>(partial);
}

#[test]
fn overlapping_group_leaf_and_root_requests_release_only_their_own_demand() {
	let mut app = app();
	let label = app
		.world_mut()
		.spawn((
			Text::new("stale"),
			LocalizedText::from(Message::new(|hud: &texts::presentation::Hud| {
				hud.msg_title()
			})),
		))
		.id();
	app.update();
	assert!(!app.world().contains_resource::<texts::presentation::Hud>());
	assert!(app.world().get::<Text>(label).unwrap().0.is_empty());
	{
		let mut localization = app
			.world_mut()
			.resource_mut::<Localization<Translations, Lazy>>();
		assert_eq!(
			localization.status::<Translations>(),
			ModuleStatus::Unloaded
		);
		assert!(localization.modules().presentation().hud().is_err());
		localization.load::<texts::Presentation>();
		localization.load::<texts::presentation::Hud>();
	}

	app.update();
	let localization = app.world().resource::<Localization<Translations, Lazy>>();
	let hud = localization.modules().presentation().hud().unwrap();
	assert_eq!(hud.msg_title(), "Flight HUD");
	assert!(localization.catalog().is_none());
	assert!(app.world().contains_resource::<texts::Presentation>());
	assert!(!app.world().contains_resource::<Translations>());
	let retained = hud.clone();
	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.unload::<texts::Presentation>();
	app.update();
	assert!(app.world().contains_resource::<texts::presentation::Hud>());
	assert!(
		!app.world()
			.contains_resource::<texts::presentation::Panel>()
	);
	assert!(!app.world().contains_resource::<texts::Presentation>());
	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.unload::<texts::presentation::Hud>();
	app.update();
	assert!(!app.world().contains_resource::<texts::presentation::Hud>());
	assert!(app.world().get::<Text>(label).unwrap().0.is_empty());
	assert_eq!(retained.msg_title(), "Flight HUD");
	{
		let mut localization = app
			.world_mut()
			.resource_mut::<Localization<Translations, Lazy>>();
		localization.load::<Translations>();
		localization.set_locale(Locale::Es);
	}

	app.update();
	assert_eq!(app.world().resource::<Translations>().locale(), Locale::Es);
	assert_eq!(app.world().get::<Text>(label).unwrap().0, "Panel de vuelo");
	app.world_mut()
		.resource_mut::<Localization<Translations, Lazy>>()
		.unload::<Translations>();
	app.update();
	assert!(!app.world().contains_resource::<Translations>());
	assert!(!app.world().contains_resource::<texts::Presentation>());
	assert!(!app.world().contains_resource::<texts::presentation::Hud>());
}
