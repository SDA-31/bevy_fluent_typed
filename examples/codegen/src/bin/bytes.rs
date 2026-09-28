//! Generated accessors with application-supplied bytes; no runtime manifest.
use bevy_fluent_typed::bevy::prelude::*;
use bevy_fluent_typed::{Localization, LocalizationPlugin};

bevy_fluent_typed::translations!(mod texts);

fn main() {
	let plugin = LocalizationPlugin::<texts::Translations>::from_bytes([
		(
			texts::Locale::En,
			texts::ui::Greeting::PATH,
			include_bytes!("../../assets/localizations/translations/en/ui/greeting.ftl").as_slice(),
		),
		(
			texts::Locale::Es,
			texts::ui::Greeting::PATH,
			include_bytes!("../../assets/localizations/translations/es/ui/greeting.ftl").as_slice(),
		),
		(
			texts::Locale::Ru,
			texts::ui::Greeting::PATH,
			include_bytes!("../../assets/localizations/translations/ru/ui/greeting.ftl").as_slice(),
		),
	])
	.expect("known locale/module keys");
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();

	for locale in [texts::Locale::En, texts::Locale::Es, texts::Locale::Ru] {
		app.world_mut()
			.resource_mut::<Localization<texts::Translations>>()
			.set_locale(locale);
		let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);

		loop {
			app.update();

			if let Some(greeting) = app.world().get_resource::<texts::ui::Greeting>() {
				// Publication removed the previous-language resource before this read.
				println!("{}", greeting.msg_hello());
				break;
			}

			assert!(
				std::time::Instant::now() < deadline,
				"translation load timed out"
			);
			std::thread::sleep(std::time::Duration::from_millis(1));
		}
	}
}
