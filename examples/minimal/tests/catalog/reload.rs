//! Real filesystem watcher regression across modules and active/inactive locales.
use crate::{Locale, Translations, texts};
use localization_runtime::bevy::{ecs as bevy_ecs, prelude::*};
use localization_runtime::{
	CatalogUpdate, CatalogUpdateReader, FluentCatalog, Localization, LocalizationManifest,
	LocalizationPlugin, LocalizationSystems, LocalizedText,
};
use std::{
	fs,
	path::Path,
	time::{Duration, Instant},
};

#[derive(Resource, Default)]
struct Outcomes {
	loaded: Vec<Locale>,
	rejected: Vec<Locale>,
}

fn record(mut events: CatalogUpdateReader<Translations>, mut outcomes: ResMut<Outcomes>) {
	for event in events.read() {
		match event {
			CatalogUpdate::Loaded { locale, .. } => outcomes.loaded.push(*locale),
			CatalogUpdate::Rejected {
				locale: Some(locale),
				..
			} => outcomes.rejected.push(*locale),
			CatalogUpdate::Rejected {
				locale: None,
				error,
				..
			} => panic!("unexpected aggregate failure: {error}"),
		}
	}
}

fn replace_title(root: &Path, locale: Locale, module: &str, title: &str) {
	let modules = super::modules(locale);
	let original = modules.iter().find(|entry| entry.0 == module).unwrap();
	let (_, rest) = original.1.split_once('\n').unwrap();
	fs::write(
		root.join("localizations/translations")
			.join(locale.as_ref())
			.join(module),
		format!("title = {title}\n{rest}"),
	)
	.unwrap();
}

fn assert_coherent(world: &mut World) {
	let Some(root) = world.get_resource::<Translations>() else {
		assert!(world.query::<&Text>().single(world).unwrap().0.is_empty());
		assert!(world.query::<&Text2d>().single(world).unwrap().0.is_empty());
		return;
	};
	let hud = world.resource::<texts::presentation::Hud>();
	let panel = world.resource::<texts::presentation::Panel>();
	assert!(std::ptr::eq(&**hud, &**root.presentation().hud()));
	assert!(std::ptr::eq(&**panel, &**root.presentation().panel()));
	let expected_ui = hud.msg_title();
	let expected_world = panel.msg_title();
	assert_eq!(world.query::<&Text>().single(world).unwrap().0, expected_ui);
	assert_eq!(
		world.query::<&Text2d>().single(world).unwrap().0,
		expected_world
	);
}

fn pump(app: &mut App, ready: impl Fn(&World) -> bool) {
	let deadline = Instant::now() + Duration::from_secs(15);

	loop {
		app.update();
		assert_coherent(app.world_mut());

		if ready(app.world()) {
			return;
		}

		assert!(
			Instant::now() < deadline,
			"watcher did not converge before timeout"
		);
		std::thread::sleep(Duration::from_millis(10));
	}
}

#[test]
fn watched_modules_publish_independently_and_locale_switch_drops_previous_snapshots() {
	let fixture = tempfile::tempdir().unwrap();
	let root = fixture.path();
	// macOS /var is a symlink to /private/var. Watcher events use the physical
	// Unix path; both the asset root and writes must use that same spelling.
	// Keep Windows paths unchanged to avoid introducing verbatim path prefixes.
	#[cfg(unix)]
	let canonical_root = root.canonicalize().unwrap();
	#[cfg(unix)]
	let root = canonical_root.as_path();

	fs::create_dir_all(root.join("localizations")).unwrap();
	fs::write(root.join(texts::CATALOG_ASSET_PATH), texts::CATALOG_CONFIG).unwrap();

	for &locale in Translations::locales() {
		for (module, source) in super::modules(locale) {
			let path = root
				.join("localizations/translations")
				.join(locale.as_ref())
				.join(module);
			fs::create_dir_all(path.parent().unwrap()).unwrap();
			fs::write(path, source).unwrap();
		}
	}

	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: root.to_str().unwrap().to_owned(),
			watch_for_changes_override: Some(true),
			..default()
		},
	))
	.add_plugins(LocalizationPlugin::<Translations>::new(
		LocalizationManifest::parse(texts::CATALOG_CONFIG, texts::CATALOG_ASSET_PATH).unwrap(),
	))
	.init_resource::<Outcomes>()
	.add_systems(PreUpdate, record.after(LocalizationSystems::Publish));
	app.world_mut().spawn((
		Text::default(),
		LocalizedText::<Translations>::new(|catalog| catalog.presentation().hud().msg_title()),
	));
	app.world_mut().spawn((
		Text2d::default(),
		LocalizedText::<Translations>::new(|catalog| catalog.presentation().panel().msg_title()),
	));
	app.finish();
	app.cleanup();
	pump(&mut app, |world| world.contains_resource::<Translations>());

	// Separate writes may publish intermediate valid snapshots; eventual convergence
	// is required, not a fictitious multi-file filesystem transaction.
	replace_title(root, Locale::En, "presentation/hud.ftl", "Updated HUD");
	replace_title(root, Locale::En, "presentation/panel.ftl", "Updated panel");
	pump(&mut app, |world| {
		world.resource::<texts::presentation::Hud>().msg_title() == "Updated HUD"
			&& world.resource::<texts::presentation::Panel>().msg_title() == "Updated panel"
	});
	let previous = app.world().resource::<texts::presentation::Hud>().clone();
	let untouched = app
		.world()
		.get_resource_ref::<texts::Ui>()
		.unwrap()
		.last_changed();
	app.world_mut().resource_mut::<Outcomes>().loaded.clear();
	app.world_mut().resource_mut::<Outcomes>().rejected.clear();

	// One invalid language must not prevent another language from accepting edits.
	fs::write(
		root.join("localizations/translations/en/presentation/hud.ftl"),
		"title = {\n",
	)
	.unwrap();
	replace_title(root, Locale::En, "presentation/panel.ftl", "Pending panel");
	replace_title(root, Locale::Es, "presentation/hud.ftl", "Nuevo HUD");
	replace_title(root, Locale::Es, "presentation/panel.ftl", "Nuevo panel");
	pump(&mut app, |world| {
		let outcomes = world.resource::<Outcomes>();
		outcomes.rejected.contains(&Locale::En)
			&& world.resource::<texts::presentation::Panel>().msg_title() == "Pending panel"
	});
	assert!(std::ptr::eq(
		&**app.world().resource::<texts::presentation::Hud>(),
		&*previous
	));
	assert_eq!(
		app.world()
			.resource::<texts::presentation::Panel>()
			.msg_title(),
		"Pending panel"
	);
	assert_eq!(
		app.world()
			.get_resource_ref::<texts::Ui>()
			.unwrap()
			.last_changed(),
		untouched
	);
	app.world_mut()
		.resource_mut::<Localization<Translations>>()
		.set_locale(Locale::Es);
	pump(&mut app, |world| {
		world.get_resource::<Translations>().is_some_and(|root| {
			root.locale() == Locale::Es
				&& root.presentation().hud().msg_title() == "Nuevo HUD"
				&& root.presentation().panel().msg_title() == "Nuevo panel"
		})
	});
	app.world_mut()
		.resource_mut::<Localization<Translations>>()
		.set_locale(Locale::En);

	app.world_mut().resource_mut::<Outcomes>().rejected.clear();
	pump(&mut app, |world| {
		world.resource::<Outcomes>().rejected.contains(&Locale::En)
	});
	assert!(!app.world().contains_resource::<texts::presentation::Hud>());
	assert!(!app.world().contains_resource::<Translations>());
	assert_eq!(previous.msg_title(), "Updated HUD");

	app.world_mut().resource_mut::<Outcomes>().loaded.clear();
	replace_title(root, Locale::En, "presentation/hud.ftl", "Recovered HUD");
	pump(&mut app, |world| {
		world.get_resource::<Translations>().is_some_and(|root| {
			root.presentation().hud().msg_title() == "Recovered HUD"
				&& root.presentation().panel().msg_title() == "Pending panel"
		})
	});
}
