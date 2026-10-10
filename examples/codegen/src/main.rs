use bevy_fluent_typed::bevy::{asset::AssetPlugin, prelude::*};
use bevy_fluent_typed::{Localization, LocalizationPlugin, LocalizedText};
use std::time::{Duration, Instant};

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
	app.world_mut()
		.spawn(LocalizedText::<texts::ui::Greeting>::new(|greeting| {
			greeting.msg_hello()
		}));
	app.finish();
	app.cleanup();

	// The inserted binding requests this leaf; other modules stay unloaded.
	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		let deadline = Instant::now() + Duration::from_secs(10);

		loop {
			app.update();

			if app
				.world()
				.get_resource::<texts::ui::Greeting>()
				.is_some_and(|greeting| greeting.locale() == locale)
			{
				break;
			}

			assert!(Instant::now() < deadline, "translation load timed out");
			std::thread::sleep(Duration::from_millis(1));
		}

		let greeting: &texts::ui::Greeting = app.world().resource();
		println!("{}", greeting.msg_hello());
	}
}
