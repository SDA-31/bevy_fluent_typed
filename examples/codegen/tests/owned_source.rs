//! Application-acquired buffers use generated checked parsers across locale switches.
use bevy_fluent_typed::bevy::prelude::*;
use bevy_fluent_typed::{Localization, LocalizationPlugin, Manual};
use std::time::{Duration, Instant};

bevy_fluent_typed::translations!(mod texts);

#[test]
fn owned_buffers_reach_generated_catalogs_without_an_asset_server() {
	let source = texts::embed_manifest!();
	let plugin =
		LocalizationPlugin::<texts::Translations, Manual>::from_loader(move |locale, path| {
			std::future::ready(
				source
					.read(locale.as_ref(), path)
					.map(|bytes| bytes.into_owned()),
			)
		});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();
	app.world_mut()
		.resource_mut::<Localization<texts::Translations, Manual>>()
		.load::<texts::ui::Greeting>();

	for (locale, expected) in [
		(texts::Locale::En, "Hello!"),
		(texts::Locale::Es, "¡Hola!"),
		(texts::Locale::Ru, "Привет!"),
	] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations, Manual>>()
			.set_locale(locale);
		let deadline = Instant::now() + Duration::from_secs(10);

		loop {
			app.update();

			if let Some(catalog) = app.world().get_resource::<texts::ui::Greeting>()
				&& catalog.locale() == locale
			{
				assert_eq!(catalog.msg_hello(), expected);
				break;
			}

			assert!(Instant::now() < deadline, "owned source timed out");
			std::thread::sleep(Duration::from_millis(1));
		}
	}

	assert!(!app.world().contains_resource::<AssetServer>());
}
