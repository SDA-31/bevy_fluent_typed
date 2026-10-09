use bevy::{app::AppExit, prelude::*};
use bevy_fluent_typed::{Localization, LocalizationSystems, ModuleStatus, localized};
use localization_bsn_example::{app, greeting_scene, texts};
use texts::presentation::Hud as Interface;

fn main() -> AppExit {
	app()
		.add_systems(Startup, spawn_greeting)
		.add_systems(
			PostUpdate,
			localized(report_greeting).after(LocalizationSystems::Refresh),
		)
		.add_systems(Update, report_failure)
		.run()
}

fn spawn_greeting(mut commands: Commands) {
	commands.spawn_scene(greeting_scene("Ada".into()));
}

fn report_greeting(_hud: Res<Interface>, labels: Query<&Text>, mut exit: MessageWriter<AppExit>) {
	for label in &labels {
		println!("{}", label.0);
	}

	exit.write(AppExit::Success);
}

fn report_failure(
	localization: Res<Localization<texts::Translations>>,
	mut exit: MessageWriter<AppExit>,
) {
	if let ModuleStatus::Failed(error) = localization.status::<Interface>() {
		eprintln!("{error}");
		exit.write(AppExit::error());
	}
}
