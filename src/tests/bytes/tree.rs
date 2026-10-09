//! Multi-leaf byte sources: completion ordering, overlapping demand and locale isolation.
use super::{Gate, pump};
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{
	FluentCatalog, FluentScope, Lazy, Localization, LocalizationAppExt, LocalizationPlugin, Module,
	ModuleStatus, ModuleStore, ScopeRegistration,
};
use std::sync::{
	Arc, Mutex,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Clone)]
struct Root {
	presentation: Presentation,
	other: Other,
}

#[derive(Resource, Clone)]
struct Presentation {
	hud: Hud,
	panel: Panel,
}

macro_rules! leaf {
	($name:ident, $path:literal) => {
		#[derive(Resource, Clone)]
		struct $name(Arc<String>);

		impl FluentScope for $name {
			type Catalog = Root;

			fn module_paths() -> &'static [&'static str] {
				&[$path]
			}

			fn assemble(modules: &ModuleStore<Root>) -> Option<Self> {
				modules.get::<Self>().ok().cloned()
			}
		}
	};
}

leaf!(Hud, "presentation/hud.ftl");
leaf!(Panel, "presentation/panel.ftl");
leaf!(Other, "other.ftl");

impl FluentScope for Presentation {
	type Catalog = Root;

	fn module_paths() -> &'static [&'static str] {
		&["presentation/hud.ftl", "presentation/panel.ftl"]
	}

	fn assemble(modules: &ModuleStore<Root>) -> Option<Self> {
		Some(Self {
			hud: modules.get::<Hud>().ok()?.clone(),
			panel: modules.get::<Panel>().ok()?.clone(),
		})
	}
}

impl FluentScope for Root {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&[
			"presentation/hud.ftl",
			"presentation/panel.ftl",
			"other.ftl",
		]
	}

	fn assemble(modules: &ModuleStore<Self>) -> Option<Self> {
		Some(Self {
			presentation: modules.get::<Presentation>().ok()?.clone(),
			other: modules.get::<Other>().ok()?.clone(),
		})
	}
}

fn parse(bytes: &[u8]) -> Result<Arc<String>, String> {
	String::from_utf8(bytes.to_vec())
		.map(Arc::new)
		.map_err(|error| error.to_string())
}

impl FluentCatalog for Root {
	type Locale = &'static str;
	type Modules<'a> = &'a ModuleStore<Self>;

	fn locales() -> &'static [Self::Locale] {
		&["en", "es"]
	}

	fn default_locale() -> Self::Locale {
		"en"
	}

	fn modules() -> Vec<Module<Self>> {
		vec![
			Module::new::<Hud>(Hud::module_paths()[0], |_, bytes| parse(bytes).map(Hud)),
			Module::new::<Panel>(Panel::module_paths()[0], |_, bytes| parse(bytes).map(Panel)),
			Module::new::<Other>(Other::module_paths()[0], |_, bytes| parse(bytes).map(Other)),
		]
	}

	fn scopes() -> Vec<ScopeRegistration<Self>> {
		vec![
			ScopeRegistration::new::<Root>(),
			ScopeRegistration::new::<Presentation>(),
			ScopeRegistration::new::<Hud>(),
			ScopeRegistration::new::<Panel>(),
			ScopeRegistration::new::<Other>(),
		]
	}

	fn view(modules: &ModuleStore<Self>) -> Self::Modules<'_> {
		modules
	}
}

struct Attempt {
	locale: &'static str,
	path: &'static str,
	gate: Arc<Gate>,
	payload: String,
}

#[derive(Clone, Default)]
struct Source(Arc<Mutex<Vec<Attempt>>>);

impl Source {
	fn count(&self) -> usize {
		self.0.lock().unwrap().len()
	}

	fn release<S: FluentScope<Catalog = Root>>(&self, locale: &str, attempt: usize) -> String {
		let requests = self.0.lock().unwrap();
		let request = requests
			.iter()
			.filter(|request| request.locale == locale && request.path == S::module_paths()[0])
			.nth(attempt)
			.expect("requested leaf attempt");
		request.gate.release();
		request.payload.clone()
	}
}

fn controlled() -> (App, Source) {
	let source = Source::default();
	let observed = source.clone();
	let plugin = LocalizationPlugin::<Root, Lazy>::from_loader(move |locale, path| {
		let gate = Arc::new(Gate::default());
		let mut requests = observed.0.lock().unwrap();
		let payload = format!("{locale}:{path}:{}", requests.len());
		requests.push(Attempt {
			locale,
			path,
			gate: gate.clone(),
			payload: payload.clone(),
		});

		async move {
			gate.wait().await;
			Ok::<_, String>(payload.into_bytes())
		}
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();
	assert!(!app.world().contains_resource::<AssetServer>());
	(app, source)
}

#[test]
fn out_of_order_leaves_gate_native_resources_and_release_only_unrequested_scopes() {
	let (mut app, source) = controlled();
	let group_runs = Arc::new(AtomicUsize::new(0));
	let root_runs = Arc::new(AtomicUsize::new(0));
	let group_observed = group_runs.clone();
	let root_observed = root_runs.clone();
	app.add_localized_systems(
		Update,
		(
			move |_: Res<Presentation>| {
				group_observed.fetch_add(1, Ordering::Relaxed);
			},
			move |_: Res<Root>| {
				root_observed.fetch_add(1, Ordering::Relaxed);
			},
		),
	);
	{
		let mut state = app.world_mut().resource_mut::<Localization<Root, Lazy>>();
		state.load::<Root>();
		state.load::<Presentation>();
		state.load::<Hud>();
		state.load::<Root>();
	}

	pump(&mut app, |_| source.count() == 3);
	assert_eq!(
		app.world().resource::<Localization<Root, Lazy>>().pending,
		3
	);
	source.release::<Panel>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Panel>());
	assert!(!app.world().contains_resource::<Presentation>());
	assert_eq!(group_runs.load(Ordering::Relaxed), 0);
	assert_eq!(root_runs.load(Ordering::Relaxed), 0);
	source.release::<Hud>("en", 0);
	pump(&mut app, |_| group_runs.load(Ordering::Relaxed) > 0);
	assert!(!app.world().contains_resource::<Root>());
	assert_eq!(root_runs.load(Ordering::Relaxed), 0);
	assert_eq!(
		app.world().resource::<Localization<Root, Lazy>>().pending,
		1
	);
	source.release::<Other>("en", 0);
	pump(&mut app, |_| root_runs.load(Ordering::Relaxed) > 0);
	let retained = app.world().resource::<Root>().clone();
	let hud = Arc::downgrade(&retained.presentation.hud.0);
	let panel = Arc::downgrade(&retained.presentation.panel.0);
	let other = Arc::downgrade(&retained.other.0);
	assert_eq!(
		app.world().resource::<Localization<Root, Lazy>>().pending,
		0
	);
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.unload::<Root>();
	let before = root_runs.load(Ordering::Relaxed);
	app.update();
	assert_eq!(root_runs.load(Ordering::Relaxed), before);
	assert!(!app.world().contains_resource::<Root>());
	assert!(!app.world().contains_resource::<Other>());
	assert!(app.world().contains_resource::<Presentation>());
	drop(retained);
	assert!(other.upgrade().is_none());
	assert!(panel.upgrade().is_some());
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.unload::<Presentation>();
	let before = group_runs.load(Ordering::Relaxed);
	app.update();
	assert_eq!(group_runs.load(Ordering::Relaxed), before);
	assert!(!app.world().contains_resource::<Presentation>());
	assert!(!app.world().contains_resource::<Panel>());
	assert!(app.world().contains_resource::<Hud>());
	assert!(panel.upgrade().is_none());
	assert!(hud.upgrade().is_some());
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.unload::<Hud>();
	app.update();
	assert!(hud.upgrade().is_none());
	assert!(!app.world().contains_resource::<Hud>());
	assert_eq!(
		source.count(),
		3,
		"overlapping requests must share leaf loads"
	);
}

#[test]
fn parent_unload_discards_pending_siblings_without_refetching_the_retained_leaf() {
	let (mut app, source) = controlled();
	{
		let mut state = app.world_mut().resource_mut::<Localization<Root, Lazy>>();
		state.load::<Root>();
		state.load::<Hud>();
	}

	pump(&mut app, |_| source.count() == 3);
	let hud = source.release::<Hud>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Hud>());
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.unload::<Root>();
	app.update();
	assert_eq!(
		app.world().resource::<Localization<Root, Lazy>>().pending,
		0
	);
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.load::<Root>();
	pump(&mut app, |_| source.count() == 5);
	source.release::<Panel>("en", 0);
	source.release::<Other>("en", 0);
	let other = source.release::<Other>("en", 1);
	pump(&mut app, |world| world.contains_resource::<Other>());
	assert!(!app.world().contains_resource::<Root>());
	assert!(!app.world().contains_resource::<Presentation>());
	assert!(!app.world().contains_resource::<Panel>());
	assert_eq!(
		app.world().resource::<Localization<Root, Lazy>>().pending,
		1
	);
	let panel = source.release::<Panel>("en", 1);
	pump(&mut app, |world| world.contains_resource::<Root>());
	let root = app.world().resource::<Root>();
	assert_eq!(*root.presentation.hud.0, hud);
	assert_eq!(*root.presentation.panel.0, panel);
	assert_eq!(*root.other.0, other);
	assert_eq!(source.count(), 5);
}

#[test]
fn locale_change_during_partial_loading_never_assembles_a_mixed_language_tree() {
	let (mut app, source) = controlled();
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.load::<Root>();
	pump(&mut app, |_| source.count() == 3);
	source.release::<Hud>("en", 0);
	source.release::<Panel>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Presentation>());
	let old = app.world().resource::<Presentation>().clone();
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.set_locale("es");
	pump(&mut app, |_| source.count() == 6);
	assert!(!app.world().contains_resource::<Presentation>());
	assert!(!app.world().contains_resource::<Hud>());
	assert!(!app.world().contains_resource::<Panel>());
	source.release::<Other>("en", 0);
	let panel = source.release::<Panel>("es", 0);
	let other = source.release::<Other>("es", 0);
	pump(&mut app, |world| {
		world.contains_resource::<Panel>() && world.contains_resource::<Other>()
	});
	assert!(!app.world().contains_resource::<Root>());
	assert!(!app.world().contains_resource::<Presentation>());
	let hud = source.release::<Hud>("es", 0);
	pump(&mut app, |world| world.contains_resource::<Root>());
	let root = app.world().resource::<Root>();
	assert_eq!(*root.presentation.hud.0, hud);
	assert_eq!(*root.presentation.panel.0, panel);
	assert_eq!(*root.other.0, other);
	assert!(old.hud.0.starts_with("en:"));
	assert!(old.panel.0.starts_with("en:"));
	assert_eq!(
		app.world().resource::<Localization<Root, Lazy>>().pending,
		0
	);
}

#[test]
fn full_byte_collection_publishes_complete_groups_despite_a_missing_sibling() {
	let plugin = LocalizationPlugin::<Root>::from_bytes([
		("en", Hud::module_paths()[0], b"en hud".as_slice()),
		("en", Panel::module_paths()[0], b"en panel".as_slice()),
		("es", Hud::module_paths()[0], b"es hud".as_slice()),
		("es", Panel::module_paths()[0], b"es panel".as_slice()),
		("es", Other::module_paths()[0], b"es other".as_slice()),
	])
	.unwrap();
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	app.finish();
	app.cleanup();
	pump(&mut app, |world| {
		world.contains_resource::<Presentation>()
			&& matches!(
				world.resource::<Localization<Root>>().status::<Other>(),
				ModuleStatus::Failed(_)
			)
	});
	assert!(!app.world().contains_resource::<Root>());
	assert!(!app.world().contains_resource::<Other>());
	app.world_mut()
		.resource_mut::<Localization<Root>>()
		.set_locale("es");
	pump(&mut app, |world| world.contains_resource::<Root>());
	let root = app.world().resource::<Root>();
	assert_eq!(*root.presentation.hud.0, "es hud");
	assert_eq!(*root.presentation.panel.0, "es panel");
	assert_eq!(*root.other.0, "es other");
}

#[test]
fn progress_counts_complete_schema_during_partial_group_loading_and_locale_changes() {
	let (mut app, source) = controlled();
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.load::<Hud>();
	pump(&mut app, |_| source.count() == 1);
	let state = app.world().resource::<Localization<Root, Lazy>>();
	let root = state.progress::<Root>();
	assert_eq!(root.total, 3);
	assert_eq!(root.loading, 1);
	assert_eq!(root.unloaded, 2);
	assert_eq!(root.available, 0);
	let group = state.progress::<Presentation>();
	assert_eq!(group.total, 2);
	assert_eq!(group.loading, 1);
	assert_eq!(group.unloaded, 1);
	assert_eq!(source.count(), 1, "inspection must not start sibling loads");
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.load::<Presentation>();
	pump(&mut app, |_| source.count() == 2);
	source.release::<Panel>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Panel>());
	let state = app.world().resource::<Localization<Root, Lazy>>();
	let group = state.progress::<Presentation>();
	assert_eq!(group.total, 2);
	assert_eq!(group.ready, 1);
	assert_eq!(group.loading, 1);
	assert_eq!(group.available, 1);
	assert_eq!(group.failed + group.unloaded, 0);
	assert_eq!(group.modules[0].path, Hud::module_paths()[0]);
	assert_eq!(group.modules[0].status, ModuleStatus::Loading);
	assert!(!group.modules[0].usable);
	assert_eq!(group.modules[1].path, Panel::module_paths()[0]);
	assert_eq!(group.modules[1].status, ModuleStatus::Ready);
	assert!(group.modules[1].usable);
	assert_eq!(state.progress::<Root>().total, root.total);
	source.release::<Hud>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Presentation>());
	let state = app.world().resource::<Localization<Root, Lazy>>();
	let group = state.progress::<Presentation>();
	assert_eq!(group.ready, 2);
	assert_eq!(group.available, 2);
	assert_eq!(group.loading + group.failed + group.unloaded, 0);
	assert_eq!(state.progress::<Root>().total, root.total);
	#[cfg(feature = "bevy-0-16")]
	app.world_mut()
		.send_event(crate::ReloadCatalogs::<Root>::default());
	#[cfg(not(feature = "bevy-0-16"))]
	app.world_mut()
		.write_message(crate::ReloadCatalogs::<Root>::default());
	pump(&mut app, |_| source.count() == 4);
	let group = app
		.world()
		.resource::<Localization<Root, Lazy>>()
		.progress::<Presentation>();
	assert_eq!(group.total, 2);
	assert_eq!(group.loading, 2);
	assert_eq!(group.available, 2);
	assert_eq!(group.ready + group.failed + group.unloaded, 0);
	assert!(group.modules.iter().all(|module| module.usable));
	app.world_mut()
		.resource_mut::<Localization<Root, Lazy>>()
		.set_locale("es");
	let state = app.world().resource::<Localization<Root, Lazy>>();
	let root = state.progress::<Root>();
	assert_eq!(root.locale, "es");
	assert_eq!(root.total, 3);
	assert_eq!(root.unloaded, 3);
	assert_eq!(root.available, 0);
	assert_eq!(root.ready + root.loading + root.failed, 0);
}
