use crate::{
	application::{self, Outcomes},
	texts,
};
use bevy_fluent_typed::{
	FluentCatalog, Localization, ReloadCatalogs,
	bevy::{asset::io::memory::Dir, prelude::*},
};
use std::path::Path;

const HUD: &str = "localizations/translations/en/ui/hud.ftl";
const PANEL: &str = "localizations/translations/en/ui/panel.ftl";

fn app(files: Dir) -> App {
	let mut app = application::app(files);
	// Check every completed frame, not just the final result of the asynchronous load.
	app.add_systems(Last, assert_coherent);
	app
}

fn assert_coherent(world: &mut World) {
	let root = world.resource::<texts::Translations>();
	let hud = world.resource::<texts::ui::Hud>();
	let panel = world.resource::<texts::ui::Panel>();
	assert!(std::ptr::eq(&**hud, &**root.ui().hud()));
	assert!(std::ptr::eq(&**panel, &**root.ui().panel()));
	let hud = hud.msg_title();
	let panel = panel.msg_title();
	assert_eq!(world.query::<&Text>().single(world).unwrap().0, hud);
	assert_eq!(world.query::<&Text2d>().single(world).unwrap().0, panel);
}

fn reload(app: &mut App) {
	*app.world_mut().resource_mut::<Outcomes>() = Default::default();
	app.world_mut()
		.write_message(ReloadCatalogs::<texts::Translations>::default());
	application::wait_for_load(app);
}

#[test]
fn named_source_publishes_generated_resources_and_existing_text_without_watching() {
	let files = application::source_files();
	files.insert_asset_text(Path::new(HUD), "title = External HUD\n");
	files.insert_asset_text(Path::new(PANEL), "title = External panel\n");
	let mut app = app(files.clone());
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Ready"
	);
	application::wait_for_load(&mut app);
	assert_eq!(app.world().resource::<Outcomes>().loaded.len(), 3);
	assert!(app.world().resource::<Outcomes>().rejected.is_empty());
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"External HUD"
	);
	assert_eq!(
		app.world().resource::<texts::ui::Panel>().msg_title(),
		"External panel"
	);
	let unchanged = app.world().resource::<texts::ui::Hud>().clone();
	reload(&mut app);
	assert!(std::ptr::eq(
		&*unchanged,
		&**app.world().resource::<texts::ui::Hud>()
	));

	files.insert_asset_text(Path::new(HUD), include_str!("../updates/en/hud.ftl"));
	reload(&mut app);
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Ready to explore"
	);
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Ru);
	app.update();
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Готово"
	);
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::En);
	app.update();
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Ready to explore"
	);
}

#[test]
fn invalid_language_retains_all_its_modules_while_another_language_updates() {
	let files = application::source_files();
	files.insert_asset_text(Path::new(HUD), "title = Last good HUD\n");
	let mut app = app(files.clone());
	application::wait_for_load(&mut app);
	let previous = app.world().resource::<texts::ui::Hud>().clone();
	files.insert_asset_text(Path::new(HUD), "title = Pending HUD\n");
	files.insert_asset_text(Path::new(PANEL), "title = { $unexpected }\n");
	files.insert_asset_text(
		Path::new("localizations/translations/es/ui/hud.ftl"),
		"title = Actualizado\n",
	);
	reload(&mut app);
	assert_eq!(
		app.world().resource::<Outcomes>().rejected,
		[Some(texts::Locale::En)]
	);
	assert!(std::ptr::eq(
		&*previous,
		&**app.world().resource::<texts::ui::Hud>()
	));
	assert_eq!(
		app.world().resource::<texts::ui::Panel>().msg_title(),
		"Equipment"
	);
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Es);
	app.update();
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Actualizado"
	);

	files.insert_asset_text(Path::new(PANEL), "title = Recovered panel\n");
	reload(&mut app);
	assert!(app.world().resource::<Outcomes>().rejected.is_empty());
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Actualizado"
	);
	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::En);
	app.update();
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Pending HUD"
	);
	assert_eq!(
		app.world().resource::<texts::ui::Panel>().msg_title(),
		"Recovered panel"
	);
}

#[test]
fn missing_source_data_at_start_can_be_installed_and_explicitly_retried() {
	let files = Dir::default();
	let mut app = app(files.clone());
	application::wait_for_load(&mut app);
	assert_eq!(app.world().resource::<Outcomes>().rejected, [None]);
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Ready"
	);
	files.insert_asset_text(Path::new(texts::CATALOG_ASSET_PATH), texts::CATALOG_CONFIG);

	for (locale, path, source) in texts::MODULES {
		files.insert_asset_text(
			&Path::new("localizations/translations")
				.join(locale)
				.join(path),
			source,
		);
	}

	files.insert_asset_text(Path::new(HUD), "title = Installed later\n");
	reload(&mut app);
	assert!(app.world().resource::<Outcomes>().rejected.is_empty());
	assert_eq!(
		app.world().resource::<Outcomes>().loaded.len(),
		texts::Translations::locales().len()
	);
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Installed later"
	);
}

#[test]
fn definition_missing_module_and_non_utf8_failures_preserve_last_good_resources() {
	let files = application::source_files();
	files.insert_asset_text(Path::new(HUD), "title = Last good HUD\n");
	let mut app = app(files.clone());
	application::wait_for_load(&mut app);
	files.insert_asset_text(Path::new(texts::CATALOG_ASSET_PATH), "invalid TOML");
	reload(&mut app);
	assert_eq!(app.world().resource::<Outcomes>().rejected, [None]);
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Last good HUD"
	);

	files.insert_asset_text(Path::new(texts::CATALOG_ASSET_PATH), texts::CATALOG_CONFIG);
	files.remove_asset(Path::new(PANEL));
	reload(&mut app);
	assert_eq!(
		app.world().resource::<Outcomes>().rejected,
		[Some(texts::Locale::En)]
	);
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Last good HUD"
	);

	files.insert_asset(Path::new(PANEL), vec![0xff]);
	reload(&mut app);
	assert_eq!(
		app.world().resource::<Outcomes>().rejected,
		[Some(texts::Locale::En)]
	);
	assert_eq!(
		app.world().resource::<texts::ui::Hud>().msg_title(),
		"Last good HUD"
	);

	files.insert_asset_text(Path::new(PANEL), "title = Recovered panel\n");
	reload(&mut app);
	assert!(app.world().resource::<Outcomes>().rejected.is_empty());
	assert_eq!(
		app.world().resource::<texts::ui::Panel>().msg_title(),
		"Recovered panel"
	);
}
