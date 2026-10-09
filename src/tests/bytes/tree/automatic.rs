use super::{Hud, Other, Panel, Presentation, Root, controlled_mode, pump};
use crate::bevy::prelude::*;
use crate::{Auto, LocalizationAppExt, LocalizedText};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[test]
fn automatic_group_and_leaf_owners_load_only_their_union_and_release_independently() {
	let (mut app, source) = controlled_mode::<Auto>();
	let hud = app
		.world_mut()
		.spawn(LocalizedText::<Hud>::new(|hud| hud.0.to_string()))
		.id();
	let group = app
		.world_mut()
		.spawn(LocalizedText::<Presentation>::new(|group| {
			group.panel.0.to_string()
		}))
		.id();
	pump(&mut app, |_| source.count() == 2);
	source.release::<Hud>("en", 0);
	source.release::<Panel>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Presentation>());
	assert!(!app.world().contains_resource::<Other>());
	assert!(!app.world().contains_resource::<Root>());
	app.world_mut().despawn(group);
	app.update();
	assert!(app.world().contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Panel>());
	app.world_mut().despawn(hud);
	app.update();
	assert!(!app.world().contains_resource::<Hud>());
	assert_eq!(source.count(), 2);
}

#[test]
fn required_leaf_system_loads_only_its_module_without_manual_requests() {
	let (mut app, source) = controlled_mode::<Auto>();
	let runs = Arc::new(AtomicUsize::new(0));
	let observed = runs.clone();
	app.add_localized_systems(Update, move |_: Res<Hud>| {
		observed.fetch_add(1, Ordering::Relaxed);
	});
	pump(&mut app, |_| source.count() == 1);
	source.release::<Hud>("en", 0);
	pump(&mut app, |_| runs.load(Ordering::Relaxed) > 0);
	assert!(app.world().contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Panel>());
	assert!(!app.world().contains_resource::<Other>());
	assert_eq!(source.count(), 1);
}

#[cfg(feature = "manifest")]
#[test]
fn manifest_only_plugin_and_required_leaf_consumer_leave_siblings_unloaded() {
	use crate::{Localization, LocalizationManifest, LocalizationPlugin, ModuleStatus};

	let manifest = LocalizationManifest::__embedded((
		"en",
		"en",
		".",
		&[
			("en", "presentation/hud.ftl", b"hud"),
			("en", "presentation/panel.ftl", b"panel"),
			("en", "other.ftl", b"other"),
		],
	));
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		LocalizationPlugin::<Root>::new(manifest),
	))
	.add_localized_systems(Update, |hud: Res<Hud>| assert_eq!(&**hud.0, "hud"));
	app.update();
	app.update();
	assert!(app.world().contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Panel>());
	assert!(!app.world().contains_resource::<Other>());
	assert_eq!(
		app.world()
			.resource::<Localization<Root>>()
			.status::<Panel>(),
		ModuleStatus::Unloaded
	);
}
