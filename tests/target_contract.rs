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
fn default_initializes_once_and_does_not_revive_an_application_removed_target() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = app(policy);
		let calls = Arc::new(AtomicUsize::new(0));
		let entity = app
			.world_mut()
			.spawn(Localized::<Text, _>::from(message(&calls)))
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
		assert!(app.world().get::<Text>(entity).is_some());
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
fn minimal_non_default_adapter_waits_for_the_application_component() {
	let mut app = app(CachePolicy::None);
	let calls = Arc::new(AtomicUsize::new(0));
	app.world_mut().insert_resource(Catalog("ready".into()));
	let entity = app
		.world_mut()
		.spawn(Localized::<ApplicationCaption, _>::from(message(&calls)))
		.id();
	app.update();
	assert!(app.world().get::<ApplicationCaption>(entity).is_none());
	assert_eq!(calls.load(Ordering::Relaxed), 0);
	app.world_mut()
		.entity_mut(entity)
		.insert(ApplicationCaption(String::new()));
	app.update();
	assert_eq!(
		app.world().get::<ApplicationCaption>(entity).unwrap().0,
		"ready"
	);
}

#[cfg(feature = "derive")]
mod derived {
	use super::{Catalog, app, message};
	use bevy_fluent_typed::bevy::{ecs as bevy_ecs, prelude::Component};
	use bevy_fluent_typed::{CachePolicy, LocalizationTarget, Localized};
	use std::{
		marker::PhantomData,
		sync::{
			Arc,
			atomic::{AtomicUsize, Ordering},
		},
	};

	#[derive(Component, LocalizationTarget)]
	struct Caption<T: Send + Sync + 'static> {
		#[localized]
		text: String,
		style: u32,
		marker: PhantomData<T>,
	}

	impl<T: Send + Sync + 'static> Default for Caption<T> {
		fn default() -> Self {
			Self {
				text: String::new(),
				style: 42,
				marker: PhantomData,
			}
		}
	}

	struct NoDefaultArgument;

	#[test]
	fn derive_supports_manual_default_generics_and_preserves_supplied_settings() {
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
					style: 7,
					..Default::default()
				},
				Localized::<Caption<NoDefaultArgument>, _>::from(message(&calls)),
			))
			.id();
		app.update();
		assert_eq!(
			app.world()
				.get::<Caption<NoDefaultArgument>>(implicit)
				.unwrap()
				.style,
			42
		);
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
