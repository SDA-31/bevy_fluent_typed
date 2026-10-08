use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{Localization, LocalizationPlugin};

bevy_fluent_typed::translations!(mod texts);

texts::embed_manifest! {
	const EMBEDDED = Translations;
}

fn main() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<texts::Translations>::new(EMBEDDED),
	));
	app.finish();
	app.cleanup();

	// Explicit embedded sources are parsed during the next plugin update.
	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		app.update();
		let greeting: &texts::ui::Greeting = app.world().resource();
		println!("{}", greeting.msg_hello());
	}
}
