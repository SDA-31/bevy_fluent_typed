use bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{Localization, LocalizationPlugin, LocalizedText};

bevy_fluent_typed::translations!(pub mod texts);

fn main() {
	let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(texts::ASSET_ROOT);
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: assets.to_string_lossy().into_owned(),
			..default()
		},
	))
	.add_plugins(LocalizationPlugin::<texts::Translations>::new(
		texts::CATALOG_ASSET_PATH,
	))
	.add_systems(Startup, show_hud);
	app.finish();
	app.cleanup();
	app.update();

	// Typed borrows: whole tree -> folder -> file.
	let translations: &texts::Translations = app.world().resource();
	let presentation: &texts::Presentation = translations.presentation();
	let hud: &texts::presentation::Hud = presentation.hud();
	assert_eq!(hud.msg_title(), "Flight HUD");

	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Es);
	app.update();

	// Direct resources and existing text bindings follow the same language switch.
	let hud: &texts::presentation::Hud = app.world().resource();
	assert_eq!(hud.msg_title(), "Panel de vuelo");
	println!("{}", hud.msg_title());

	let mut labels = app.world_mut().query::<&Text>();
	let label = labels.single(app.world()).expect("one localized label");
	assert!(label.0.contains("Piloto"));
}

fn show_hud(mut commands: Commands, hud: Res<texts::presentation::Hud>) {
	println!("{}: {}", hud.msg_title(), hud.msg_detail("Ada"));

	commands.spawn((
		Text::default(),
		LocalizedText::<texts::Translations>::new(|catalog| {
			catalog.presentation().hud().msg_detail("Ada")
		}),
	));
}
