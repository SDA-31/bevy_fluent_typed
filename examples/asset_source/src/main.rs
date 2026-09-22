use bevy_fluent_typed::{Localization, ReloadCatalogs};
use std::path::Path;

mod application;

#[cfg(test)]
mod tests;

bevy_fluent_typed::translations!(mod texts);

fn main() {
	let files = application::source_files();
	let mut app = application::app(files.clone());
	application::wait_for_load(&mut app);
	println!("{}", app.world().resource::<texts::ui::Hud>().msg_title());

	// An archive-backed source would install a complete pack here. This example
	// changes one virtual file only after the previous load has completed.
	files.insert_asset_text(
		Path::new("localizations/translations/en/ui/hud.ftl"),
		include_str!("../updates/en/hud.ftl"),
	);
	*app.world_mut().resource_mut::<application::Outcomes>() = Default::default();
	app.world_mut()
		.write_message(ReloadCatalogs::<texts::Translations>::default());
	application::wait_for_load(&mut app);
	println!("{}", app.world().resource::<texts::ui::Hud>().msg_title());

	app.world_mut()
		.resource_mut::<Localization<texts::Translations>>()
		.set_locale(texts::Locale::Es);
	app.update();
	println!("{}", app.world().resource::<texts::ui::Hud>().msg_title());
}
