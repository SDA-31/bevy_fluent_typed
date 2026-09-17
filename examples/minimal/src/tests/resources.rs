use crate::{Locale, Translations, texts};
use localization_runtime::bevy::{ecs as bevy_ecs, prelude::*};
use localization_runtime::{FluentCatalog, Localization, LocalizationPlugin, LocalizationSystems};

#[derive(Resource, Default)]
struct Observations(Vec<(String, bool)>);

fn observe(hud: Res<texts::presentation::Hud>, mut observations: ResMut<Observations>) {
	observations.0.push((hud.msg_title(), hud.is_changed()));
}

fn app() -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.insert_resource(Localization::<Translations>::new(Locale::En))
		.init_resource::<Observations>()
		.add_plugins(LocalizationPlugin::<Translations>::new(
			"not-loaded-in-resource-test.toml",
		));
	app.finish();
	app.cleanup();
	app
}

#[test]
fn group_chain_and_direct_resources_share_data_before_startup_and_after_switch() {
	let mut app = app();
	app.add_systems(Update, observe);

	for &locale in Translations::locales() {
		if locale != Locale::En {
			app.world_mut()
				.resource_mut::<Localization<Translations>>()
				.set_locale(locale);
			app.update();
		}

		let root: &Translations = app.world().resource();
		let presentation: &texts::Presentation = app.world().resource();
		let direct: &texts::presentation::Hud = app.world().resource();
		let nested: &texts::presentation::Hud = root.presentation().hud();
		let catalog: &texts::Catalog = app.world().resource();
		assert_eq!(root.locale(), locale);
		assert_eq!(catalog.msg_title(), root.catalog().msg_title());
		assert_eq!(direct.msg_title(), presentation.hud().msg_title());
		assert!(
			std::ptr::eq(&**direct, &**nested),
			"parsed bundles should be shared, not cloned"
		);
	}

	app.update();
	assert!(!app.world().resource::<Observations>().0.last().unwrap().1);
	let unchanged_locale = app
		.world()
		.resource::<Localization<Translations>>()
		.locale();

	app.world_mut()
		.resource_mut::<Localization<Translations>>()
		.set_locale(unchanged_locale);
	app.update();
	assert!(!app.world().resource::<Observations>().0.last().unwrap().1);
}

#[test]
fn an_update_language_change_is_published_before_post_update_consumers() {
	fn switch(mut texts: ResMut<Localization<Translations>>) {
		texts.set_locale(Locale::Es);
	}

	let mut app = app();
	app.add_systems(Update, switch)
		.add_systems(PostUpdate, observe.after(LocalizationSystems::Refresh));
	app.update();
	assert_eq!(
		app.world().resource::<Observations>().0[0].0,
		"Panel de vuelo"
	);
}

#[cfg(feature = "bevy-0-19")]
#[test]
fn generated_root_group_and_leaf_are_ecs_immutable_on_bevy_019() {
	use localization_runtime::bevy::ecs::component::Immutable;

	fn immutable<T: Resource + Component<Mutability = Immutable>>() {}

	immutable::<Translations>();
	immutable::<texts::Presentation>();
	immutable::<texts::presentation::Hud>();
}

#[test]
fn duplicate_local_keys_have_independent_types_references_attributes_and_structured_results() {
	let catalog = Locale::En.load();
	let presentation: &texts::Presentation = catalog.presentation();
	let hud: &texts::presentation::Hud = presentation.hud();
	let panel: &texts::presentation::Panel = presentation.panel();
	assert_eq!(hud.msg_title(), "Flight HUD");
	assert_eq!(panel.msg_title(), "Settings panel");
	assert!(hud.msg_detail("Ada").contains("Ada"));
	assert!(panel.msg_detail(42).contains("42"));
	assert_eq!(hud.msg_caption(), hud.msg_title());
	assert_eq!(hud.msg_caption_hint(), "Visible HUD");
	let prompt: texts::presentation::HudPrompt = hud.prompt();
	assert!(prompt.s0.contains("Press"));
	let raw = super::raw::Locale::En.load();
	let raw_prompt: super::raw::presentation::HudPrompt = raw.presentation().hud().prompt();
	assert_eq!(prompt.s0, raw_prompt.s0);
}

#[test]
fn module_paths_are_order_independent_and_missing_extra_or_duplicate_inputs_are_rejected() {
	let mut modules: Vec<_> = Translations::modules(Locale::En)
		.into_iter()
		.map(|module| (module.path, module.embedded))
		.collect();
	modules.reverse();
	assert_eq!(
		Translations::from_modules(Locale::En, &modules)
			.unwrap()
			.presentation()
			.hud()
			.msg_title(),
		"Flight HUD"
	);
	let removed = modules.pop().unwrap();
	assert!(
		Translations::from_modules(Locale::En, &modules)
			.err()
			.unwrap()
			.contains("missing")
	);
	modules.push(removed);
	modules.push(removed);
	assert!(
		Translations::from_modules(Locale::En, &modules)
			.err()
			.unwrap()
			.contains("duplicate")
	);
	modules.pop();
	modules.push(("unexpected.ftl", "title = Surprise"));
	assert!(
		Translations::from_modules(Locale::En, &modules)
			.err()
			.unwrap()
			.contains("unexpected")
	);
}
