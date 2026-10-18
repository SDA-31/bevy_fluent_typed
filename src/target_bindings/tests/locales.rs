//! Fixed and following bindings share the public adapter and source ownership contracts.
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{App, Component, EntityWorldMut, MinimalPlugins, Text},
};
use crate::tests::{TestCatalog, pump};
use crate::{
	CachePolicy, Full, Localization, LocalizationPlugin, LocalizationTarget, LocalizationUpdate,
	Localized, Manual, Msg,
};
use std::sync::{
	Arc, Mutex,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Component, Default)]
struct Caption(String);

impl LocalizationTarget for Caption {
	type Data = &'static mut Self;
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get_mut::<Self>() else {
			return;
		};
		update.set_text(target.map_unchanged(|target| &mut target.0));
	}
}

#[test]
fn fixed_and_following_texts_share_acquisition_and_keep_independent_updates() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let reads = Arc::new(Mutex::new(Vec::new()));
		let observed = reads.clone();
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			LocalizationPlugin::<TestCatalog>::from_loader(move |locale, _| {
				observed.lock().unwrap().push(locale);
				std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
			})
			.with_cache_policy(policy),
		));
		let message = Msg::<TestCatalog>::new(|scope| scope.0.clone());
		let following = app
			.world_mut()
			.spawn((Text::default(), Localized::<Text, _>::from(message.clone())))
			.id();
		let fixed = app
			.world_mut()
			.spawn((
				Text::default(),
				Localized::<Text, _>::from(message.clone()).with_locale("es"),
			))
			.id();
		let custom = app
			.world_mut()
			.spawn((
				Caption::default(),
				Localized::<Caption, _>::from(message.clone()).with_locale("es"),
			))
			.id();
		pump(&mut app, |world| {
			world.get::<Text>(following).unwrap().0 == "ja"
				&& world.get::<Text>(fixed).unwrap().0 == "es"
				&& world.get::<Caption>(custom).unwrap().0 == "es"
		});
		assert_eq!(reads.lock().unwrap().len(), 2);
		app.world_mut()
			.resource_mut::<Localization<TestCatalog>>()
			.set_locale("es");
		pump(&mut app, |world| {
			world.get::<Text>(following).unwrap().0 == "es"
		});
		assert_eq!(reads.lock().unwrap().len(), 2);
		assert_eq!(app.world().get::<Caption>(custom).unwrap().0, "es");

		app.world_mut()
			.resource_mut::<Localization<TestCatalog>>()
			.set_locale("de");
		pump(&mut app, |world| {
			world.get::<Text>(following).unwrap().0 == "de"
		});
		assert_eq!(app.world().get::<Text>(fixed).unwrap().0, "es");
		assert_eq!(app.world().get::<Caption>(custom).unwrap().0, "es");
		app.world_mut().despawn(fixed);
		app.update();
		assert!(
			app.world()
				.resource::<Localization<TestCatalog>>()
				.for_locale("es")
				.catalog()
				.is_some()
		);
		app.world_mut().despawn(custom);
		app.update();
		assert!(
			app.world()
				.resource::<Localization<TestCatalog>>()
				.for_locale("es")
				.catalog()
				.is_none()
		);
	}
}

#[test]
fn replacing_only_the_selection_with_the_same_msg_invalidates_text_and_demand() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<TestCatalog>::from_loader(|locale, _| {
			std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
		}),
	));
	let message = Msg::<TestCatalog>::new(|scope| scope.0.clone());
	let target = app
		.world_mut()
		.spawn((
			Text::default(),
			Localized::<Text, _>::from(message.clone()).with_locale("es"),
		))
		.id();
	pump(&mut app, |world| {
		world.get::<Text>(target).unwrap().0 == "es"
	});
	assert!(!app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.entity_mut(target)
		.insert(Localized::<Text, _>::from(message.clone()).with_locale("de"));
	pump(&mut app, |world| {
		world.get::<Text>(target).unwrap().0 == "de"
	});
	assert!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.for_locale("es")
			.catalog()
			.is_none()
	);
	app.world_mut().entity_mut(target).insert(
		Localized::<Text, _>::from(message)
			.with_locale("de")
			.follow_locale(),
	);
	pump(&mut app, |world| {
		world.get::<Text>(target).unwrap().0 == "ja"
	});
	assert!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.for_locale("de")
			.catalog()
			.is_none()
	);
}

#[test]
fn fixed_native_text_does_not_format_on_idle_or_unrelated_active_switches() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = calls.clone();
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			LocalizationPlugin::<TestCatalog>::from_loader(|locale, _| {
				std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
			})
			.with_cache_policy(policy),
		));
		let target = app
			.world_mut()
			.spawn((
				Text::default(),
				Localized::<Text, TestCatalog>::new(move |scope| {
					observed.fetch_add(1, Ordering::Relaxed);
					scope.0.clone()
				})
				.with_locale("es"),
			))
			.id();
		pump(&mut app, |world| {
			world.get::<Text>(target).unwrap().0 == "es"
		});
		let before = calls.load(Ordering::Relaxed);

		for _ in 0..6 {
			app.update();
		}

		assert_eq!(calls.load(Ordering::Relaxed), before);
		app.world_mut()
			.resource_mut::<Localization<TestCatalog>>()
			.set_locale("de");
		pump(&mut app, |world| {
			world.resource::<Localization<TestCatalog>>().locale() == "de"
		});
		assert_eq!(calls.load(Ordering::Relaxed), before);
		assert_eq!(app.world().get::<Text>(target).unwrap().0, "es");
	}
}

#[test]
fn manual_fixed_binding_waits_for_explicit_requests_and_uses_the_same_adapter() {
	let reads = Arc::new(AtomicUsize::new(0));
	let observed = reads.clone();
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<TestCatalog, Manual>::from_loader(move |locale, _| {
			observed.fetch_add(1, Ordering::Relaxed);
			std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
		}),
	));
	let target = app
		.world_mut()
		.spawn((
			Caption::default(),
			Localized::<Caption, TestCatalog>::new(|scope| scope.0.clone()).with_locale("es"),
		))
		.id();
	app.update();
	assert_eq!(reads.load(Ordering::Relaxed), 0);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.for_locale_mut("es")
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world.get::<Caption>(target).unwrap().0 == "es"
	});
	assert!(!app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.for_locale_mut("es")
		.unload::<TestCatalog>();
	app.update();
	assert!(app.world().get::<Caption>(target).unwrap().0.is_empty());
}

#[test]
fn full_retains_its_active_root_and_can_use_a_fixed_language_binding() {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<TestCatalog, Full>::from_loader(|locale, _| {
			std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
		}),
	));
	let target = app
		.world_mut()
		.spawn((
			Text::default(),
			Localized::<Text, TestCatalog>::new(|scope| scope.0.clone()).with_locale("es"),
		))
		.id();
	pump(&mut app, |world| {
		world.get::<Text>(target).unwrap().0 == "es" && world.contains_resource::<TestCatalog>()
	});
	assert_eq!(app.world().resource::<TestCatalog>().0, "ja");
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn native_text_and_accessible_label_on_one_entity_use_independent_languages() {
	use crate::bevy::prelude::AccessibleLabel;
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		LocalizationPlugin::<TestCatalog>::from_loader(|locale, _| {
			std::future::ready(Ok::<_, String>(locale.as_bytes().to_vec()))
		}),
	));
	let message = Msg::<TestCatalog>::new(|scope| scope.0.clone());
	let entity = app
		.world_mut()
		.spawn((
			Text::default(),
			AccessibleLabel::default(),
			Localized::<Text, _>::from(message.clone()).with_locale("es"),
			Localized::<AccessibleLabel, _>::from(message).with_locale("de"),
		))
		.id();
	pump(&mut app, |world| {
		world.get::<Text>(entity).unwrap().0 == "es"
			&& world.get::<AccessibleLabel>(entity).unwrap().0 == "de"
	});
	assert!(!app.world().contains_resource::<TestCatalog>());
}
