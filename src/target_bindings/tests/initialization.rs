use super::{state::State, tests::Catalog};
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{App, Component, EntityWorldMut, MinimalPlugins, Text},
};
use crate::bindings::Binding;
use crate::cache_policy::ProviderCachePolicy;
use crate::tests::{TestCatalog, pump};
use crate::{
	CachePolicy, Localization, LocalizationPlugin, LocalizationTarget, LocalizationUpdate,
	Localized, Msg,
};
use std::{
	any::type_name,
	process::Command,
	sync::{
		Arc, Mutex,
		atomic::{AtomicUsize, Ordering},
	},
};
use tracing::{
	Event, Metadata, Subscriber,
	span::{Attributes, Id, Record},
};

#[derive(Component)]
struct Caption {
	text: String,
	size: u32,
}

impl LocalizationTarget for Caption {
	type Data = &'static mut Self;
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get_mut::<Self>() else {
			return;
		};

		update.set_text(target.map_unchanged(|target| &mut target.text));
	}
}

#[derive(Component)]
struct DefaultTrap;

impl Default for DefaultTrap {
	fn default() -> Self {
		panic!("localization must never construct a target")
	}
}

impl LocalizationTarget for DefaultTrap {
	type Data = &'static Self;
	type Filter = ();

	fn apply(_: &mut EntityWorldMut<'_>, _: &mut LocalizationUpdate<'_>) {}
}

/// Capture warnings only inside the tested thread, without changing global logging.
struct Warnings(Arc<Mutex<Vec<String>>>);

impl Subscriber for Warnings {
	fn enabled(&self, metadata: &Metadata<'_>) -> bool {
		*metadata.level() == tracing::Level::WARN
	}

	fn new_span(&self, _: &Attributes<'_>) -> Id {
		Id::from_u64(1)
	}

	fn record(&self, _: &Id, _: &Record<'_>) {}

	fn record_follows_from(&self, _: &Id, _: &Id) {}

	fn event(&self, event: &Event<'_>) {
		self.0.lock().unwrap().push(format!("{event:?}"));
	}

	fn enter(&self, _: &Id) {}

	fn exit(&self, _: &Id) {}
}

fn rejects_missing<T: LocalizationTarget>() {
	let warnings = Arc::new(Mutex::new(Vec::new()));
	tracing::subscriber::with_default(Warnings(Arc::clone(&warnings)), || {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let world = app.world_mut();
		let capture = Arc::new(());
		let weak = Arc::downgrade(&capture);
		let binding = Localized::<T, Catalog>::new(move |_| {
			let _ = &capture;
			panic!("rejected bindings must not format")
		});
		let owner = world.spawn(binding).id();
		world.flush();

		for _ in 0..3 {
			<Localized<T, Catalog> as Binding>::refresh(world);
			world.flush();
		}

		assert!(world.get_entity(owner).is_ok());
		assert!(world.get::<T>(owner).is_none());
		assert!(world.get::<Localized<T, Catalog>>(owner).is_none());
		assert!(world.get::<State<Catalog, T>>(owner).is_none());
		assert!(weak.upgrade().is_none());
		let warnings = warnings.lock().unwrap();
		assert_eq!(warnings.len(), 1);
		assert!(warnings[0].contains("target component is missing"));
		assert!(warnings[0].contains(type_name::<T>()));
		assert!(warnings[0].contains(type_name::<Catalog>()));
		assert!(warnings[0].contains(&format!("{owner:?}")));
	});
}

#[test]
fn missing_targets_with_or_without_default_are_rejected_once() {
	// Tracing's callsite cache is process-global, even for a thread-local subscriber.
	// Keep the scoped recorder independent of concurrent tests' tracing state.
	const CHILD: &str = "BEVY_FLUENT_MISSING_TARGET_WARNING_CHILD";

	if std::env::var_os(CHILD).is_some() {
		rejects_missing::<Caption>();
		rejects_missing::<DefaultTrap>();
		return;
	}

	let module = module_path!().split_once("::").unwrap().1;
	let test = format!("{module}::missing_targets_with_or_without_default_are_rejected_once");
	let output = Command::new(std::env::current_exe().unwrap())
		.args(["--exact", &test, "--nocapture", "--color=never"])
		.env(CHILD, "1")
		.output()
		.expect("run the isolated warning test");
	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);

	assert!(
		output.status.success() && stdout.contains("1 passed; 0 failed;"),
		"isolated warning test failed: {stdout}\n{stderr}"
	);
}

#[test]
fn targets_without_default_work_with_all_constructors_under_both_cache_policies() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		let world = app.world_mut();
		world.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = Arc::clone(&calls);
		let message = Msg::<Catalog>::new(move |catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		});
		let binding = Localized::<Caption, _>::from(message.clone());
		let converted = message.into();
		let cloned = binding.clone();
		let observed = Arc::clone(&calls);
		let constructed = Localized::<Caption, Catalog>::new(move |catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		});
		let owners = [binding, cloned, constructed, converted].map(|binding| {
			world
				.spawn((
					binding,
					Caption {
						text: "supplied".into(),
						size: 48,
					},
				))
				.id()
		});
		world.flush();

		for owner in owners {
			let caption = world.get::<Caption>(owner).unwrap();
			assert_eq!(caption.text, "supplied");
			assert_eq!(caption.size, 48);
		}

		assert_eq!(calls.load(Ordering::Relaxed), 0);
		<Localized<Caption, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), 0);
		world.insert_resource(Catalog("ready".into()));
		<Localized<Caption, Catalog> as Binding>::refresh(world);

		for owner in owners {
			let caption = world.get::<Caption>(owner).unwrap();
			assert_eq!(caption.text, "ready");
			assert_eq!(caption.size, 48);
		}

		assert_eq!(calls.load(Ordering::Relaxed), owners.len());
	}
}

#[test]
fn rejected_bindings_do_not_load_catalogs_or_break_later_valid_instances() {
	for provider_first in [true, false] {
		let reads = Arc::new(AtomicUsize::new(0));
		let observed = Arc::clone(&reads);
		let plugin = LocalizationPlugin::<TestCatalog>::from_loader(move |locale, _| {
			observed.fetch_add(1, Ordering::Relaxed);
			std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
		});
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);

		if provider_first {
			app.add_plugins(plugin);
			let rejected = app
				.world_mut()
				.spawn(Localized::<Caption, TestCatalog>::new(|c| c.0.clone()))
				.id();
			app.update();
			assert!(
				app.world()
					.get::<Localized<Caption, TestCatalog>>(rejected)
					.is_none()
			);
		} else {
			let rejected = app
				.world_mut()
				.spawn(Localized::<Caption, TestCatalog>::new(|c| c.0.clone()))
				.id();
			app.world_mut().flush();
			assert!(
				app.world()
					.get::<Localized<Caption, TestCatalog>>(rejected)
					.is_none()
			);
			app.add_plugins(plugin);
			app.update();
		}

		assert_eq!(reads.load(Ordering::Relaxed), 0);
		assert!(
			app.world()
				.resource::<Localization<TestCatalog>>()
				.desired()
				.is_empty()
		);
		let owner = app
			.world_mut()
			.spawn((
				Caption {
					text: String::new(),
					size: 32,
				},
				Localized::<Caption, TestCatalog>::new(|catalog| catalog.0.clone()),
			))
			.id();
		pump(&mut app, |world| {
			world.get::<Caption>(owner).unwrap().text == "ja"
		});
		assert_eq!(reads.load(Ordering::Relaxed), 1);
		app.world_mut()
			.entity_mut(owner)
			.remove::<Localized<Caption, TestCatalog>>();
		app.update();
		assert!(!app.world().contains_resource::<TestCatalog>());
		assert!(
			app.world()
				.resource::<Localization<TestCatalog>>()
				.desired()
				.is_empty()
		);
	}
}

#[test]
fn removed_custom_targets_wait_until_reinserted_but_new_lifetimes_require_a_target() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("first".into()));
	let binding = Localized::<Caption, Catalog>::new(|catalog| catalog.0.clone());
	let owner = world
		.spawn((
			Caption {
				text: String::new(),
				size: 24,
			},
			binding.clone(),
		))
		.id();
	world.flush();
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	world.entity_mut(owner).remove::<Caption>();
	world.insert_resource(Catalog("second".into()));
	world.entity_mut(owner).insert(binding.clone());
	world.flush();
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	assert!(world.get::<Caption>(owner).is_none());
	assert!(world.get::<Localized<Caption, Catalog>>(owner).is_some());
	world.entity_mut(owner).insert(Caption {
		text: String::new(),
		size: 72,
	});
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	let caption = world.get::<Caption>(owner).unwrap();
	assert_eq!(caption.text, "second");
	assert_eq!(caption.size, 72);
	world
		.entity_mut(owner)
		.remove::<(Caption, Localized<Caption, Catalog>)>();
	world.flush();
	world.entity_mut(owner).insert(binding.clone());
	world.flush();
	assert!(world.get::<Caption>(owner).is_none());
	assert!(world.get::<Localized<Caption, Catalog>>(owner).is_none());
	world.entity_mut(owner).insert(Caption {
		text: "application".into(),
		size: 96,
	});
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<Caption>(owner).unwrap().text, "application");
	world.entity_mut(owner).insert(binding);
	world.flush();
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<Caption>(owner).unwrap().text, "second");
	assert_eq!(world.get::<Caption>(owner).unwrap().size, 96);
}

#[test]
fn target_first_and_binding_first_bundles_preserve_supplied_components() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	let binding = Localized::<Text, Catalog>::new(|catalog| catalog.0.clone());
	let first = world.spawn((Text::new("first"), binding.clone())).id();
	let second = world.spawn((binding, Text::new("second"))).id();
	world.flush();
	assert_eq!(world.get::<Text>(first).unwrap().0, "first");
	assert_eq!(world.get::<Text>(second).unwrap().0, "second");
	world.insert_resource(Catalog("ready".into()));
	<Localized<Text, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<Text>(first).unwrap().0, "ready");
	assert_eq!(world.get::<Text>(second).unwrap().0, "ready");
}
