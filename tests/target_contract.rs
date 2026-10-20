#![cfg(feature = "runtime")]

use bevy_fluent_typed::bevy::{
	ecs as bevy_ecs,
	prelude::{App, Component, EntityWorldMut, MinimalPlugins, Resource, Text},
};
use bevy_fluent_typed::{
	CachePolicy, FluentCatalog, FluentScope, LocalizationPlugin, LocalizationTarget,
	LocalizationUpdate, Localized, Module, ModuleStore, Msg, ScopeRegistration,
};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Resource, Clone)]
struct Catalog(String);

impl FluentScope for Catalog {
	type Catalog = Self;

	fn module_paths() -> &'static [&'static str] {
		&[]
	}

	fn assemble(_: &ModuleStore<Self>) -> Option<Self> {
		None
	}
}

impl FluentCatalog for Catalog {
	type Locale = &'static str;
	type Modules<'a> = ();

	fn locales() -> &'static [Self::Locale] {
		&["en"]
	}

	fn default_locale() -> Self::Locale {
		"en"
	}

	fn modules() -> Vec<Module<Self>> {
		Vec::new()
	}

	fn scopes() -> Vec<ScopeRegistration<Self>> {
		vec![ScopeRegistration::new::<Self>()]
	}

	fn view(_: &ModuleStore<Self>) -> Self::Modules<'_> {}
}

fn app(policy: CachePolicy) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<Catalog>::from_bytes::<&'static [u8]>([])
			.expect("empty catalog")
			.with_cache_policy(policy),
	));
	app.finish();
	app.cleanup();
	app
}

fn message(calls: &Arc<AtomicUsize>) -> Msg<Catalog> {
	let calls = Arc::clone(calls);
	Msg::new(move |catalog: &Catalog| {
		calls.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	})
}

#[test]
fn supplied_native_targets_are_preserved_and_removed_targets_are_not_recreated() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = app(policy);
		let calls = Arc::new(AtomicUsize::new(0));
		let entity = app
			.world_mut()
			.spawn((Text::default(), Localized::<Text, _>::from(message(&calls))))
			.id();
		app.world_mut().flush();
		assert!(app.world().get::<Text>(entity).is_some());
		assert_eq!(calls.load(Ordering::Relaxed), 0);
		app.world_mut().insert_resource(Catalog("first".into()));
		app.update();
		assert_eq!(app.world().get::<Text>(entity).unwrap().0, "first");
		app.world_mut().entity_mut(entity).remove::<Text>();
		app.world_mut().insert_resource(Catalog("second".into()));
		app.world_mut()
			.entity_mut(entity)
			.insert(Localized::<Text, _>::from(message(&calls)));
		app.update();
		assert!(app.world().get::<Text>(entity).is_none());
		app.world_mut().entity_mut(entity).insert(Text::default());
		app.update();
		assert_eq!(app.world().get::<Text>(entity).unwrap().0, "second");
		app.world_mut().entity_mut(entity).remove::<Text>();
		app.world_mut()
			.entity_mut(entity)
			.remove::<Localized<Text, Catalog>>();
		app.world_mut().flush();
		app.world_mut()
			.entity_mut(entity)
			.insert(Localized::<Text, _>::from(message(&calls)));
		app.world_mut().flush();
		assert!(app.world().get::<Text>(entity).is_none());
		assert!(
			app.world()
				.get::<Localized<Text, Catalog>>(entity)
				.is_none()
		);
	}
}

#[derive(Component)]
struct ApplicationCaption(String);

impl LocalizationTarget for ApplicationCaption {
	type Data = &'static mut Self;
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(mut target) = entity.get_mut::<Self>() else {
			return;
		};
		let text = update.text();

		if target.0 != text {
			text.clone_into(&mut target.0);
		}
	}
}

#[test]
fn minimal_custom_adapter_accepts_a_supplied_component_without_default() {
	let mut app = app(CachePolicy::None);
	let calls = Arc::new(AtomicUsize::new(0));
	app.world_mut().insert_resource(Catalog("ready".into()));
	let entity = app
		.world_mut()
		.spawn((
			ApplicationCaption(String::new()),
			Localized::<ApplicationCaption, _>::from(message(&calls)),
		))
		.id();
	app.update();
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	assert_eq!(
		app.world().get::<ApplicationCaption>(entity).unwrap().0,
		"ready"
	);
}

mod custom {
	use super::{Catalog, app, message};
	use bevy_fluent_typed::bevy::{
		ecs as bevy_ecs,
		prelude::{Component, EntityWorldMut, Mut},
	};
	use bevy_fluent_typed::{
		CachePolicy, LocalizationTarget, LocalizationUpdate, Localized, TargetItem, UpdateResult,
	};
	use std::{
		marker::PhantomData,
		sync::{
			Arc,
			atomic::{AtomicUsize, Ordering},
		},
	};

	#[derive(Component)]
	struct Caption<T: Send + Sync + 'static> {
		text: String,
		style: u32,
		marker: PhantomData<T>,
	}

	impl<T: Send + Sync + 'static> LocalizationTarget for Caption<T> {
		type Data = Mut<'static, Self>;
		type Filter = ();

		fn needs_update(data: &TargetItem<'_, '_, Self>, update: &LocalizationUpdate<'_>) -> bool {
			update.is_changed() || update.component_changed(data)
		}

		fn update(
			target: TargetItem<'_, '_, Self>,
			update: &mut LocalizationUpdate<'_>,
		) -> UpdateResult {
			update.set_text(target.map_unchanged(|target| &mut target.text));

			UpdateResult::Done
		}

		fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
			let Some(target) = entity.get_mut::<Self>() else {
				return;
			};

			update.set_text(target.map_unchanged(|target| &mut target.text));
		}
	}

	struct NoDefaultArgument;

	#[test]
	fn generic_nondefault_targets_preserve_supplied_settings_and_reject_missing_targets() {
		let mut app = app(CachePolicy::None);
		app.world_mut()
			.insert_resource(Catalog("translated".into()));
		let calls = Arc::new(AtomicUsize::new(0));
		let implicit = app
			.world_mut()
			.spawn(Localized::<Caption<NoDefaultArgument>, _>::from(message(
				&calls,
			)))
			.id();
		let supplied = app
			.world_mut()
			.spawn((
				Caption::<NoDefaultArgument> {
					text: String::new(),
					style: 7,
					marker: PhantomData,
				},
				Localized::<Caption<NoDefaultArgument>, _>::from(message(&calls)),
			))
			.id();
		app.update();
		assert!(
			app.world()
				.get::<Caption<NoDefaultArgument>>(implicit)
				.is_none()
		);
		assert!(
			app.world()
				.get::<Localized<Caption<NoDefaultArgument>, Catalog>>(implicit)
				.is_none()
		);
		assert_eq!(calls.load(Ordering::Relaxed), 1);
		assert_eq!(
			app.world()
				.get::<Caption<NoDefaultArgument>>(supplied)
				.unwrap()
				.style,
			7
		);
		assert_eq!(
			app.world()
				.get::<Caption<NoDefaultArgument>>(supplied)
				.unwrap()
				.text,
			"translated"
		);
		let formatted = calls.load(Ordering::Relaxed);
		app.update();
		assert_eq!(calls.load(Ordering::Relaxed), formatted);
	}
}
