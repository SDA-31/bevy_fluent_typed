use bevy_fluent_typed::bevy::prelude::*;
use bevy_fluent_typed::{Localization, LocalizationAppExt, LocalizationPlugin, ModuleStatus};
use std::{
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
	time::{Duration, Instant},
};

bevy_fluent_typed::translations!(mod texts);

#[test]
fn custom_loader_uses_generated_schema_checks_before_publishing_resources() {
	let mut app = App::new();
	let invoked = Arc::new(AtomicUsize::new(0));
	let observed = invoked.clone();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<texts::Translations>::from_loader(|_, _| async {
			Ok::<_, String>(b"other = Not the compiled message\n".to_vec())
		}),
	))
	.add_localized_systems(Update, move |_: Res<texts::ui::Greeting>| {
		observed.fetch_add(1, Ordering::Relaxed);
	});
	app.finish();
	app.cleanup();
	let deadline = Instant::now() + Duration::from_secs(10);

	loop {
		app.update();

		if let ModuleStatus::Failed(error) = app
			.world()
			.resource::<Localization<texts::Translations>>()
			.status::<texts::ui::Greeting>()
		{
			assert!(error.contains("hello"), "{error}");
			break;
		}

		assert!(Instant::now() < deadline, "schema check timed out");
		std::thread::sleep(Duration::from_millis(1));
	}

	assert_eq!(invoked.load(Ordering::Relaxed), 0);
	assert!(!app.world().contains_resource::<texts::ui::Greeting>());
}
