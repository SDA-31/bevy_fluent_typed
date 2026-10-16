use super::{TestCatalog, bytes::pump};
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
use crate::CachePolicy;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
use crate::bevy::prelude::{
	AccessibleLabel, AssetApp, AssetPlugin, Image, ImageNode, Query, TextureAtlasLayout,
};
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{
		App, Commands, Component, DetectChanges, Entity, EntityWorldMut, IntoScheduleConfigs,
		Local, MinimalPlugins, PostUpdate, Resource, Text, Text2d, TextSpan, With,
	},
};
use crate::{Localization, LocalizationPlugin, LocalizationTarget, Localized, Manual, Msg};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Component, Default)]
struct Caption(String);

impl LocalizationTarget for Caption {
	type Data = ();
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let Some(mut caption) = entity.get_mut::<Self>() else {
			let mut caption = Self::default();
			text.clone_into(&mut caption.0);
			entity.insert(caption);
			return;
		};

		if caption.0 == text {
			return;
		}

		text.clone_into(&mut caption.0);
	}
}

#[derive(Component, Default)]
#[component(on_add = caption_added, on_remove = caption_removed)]
struct HookedCaption {
	text: String,
}

#[derive(Resource, Default)]
struct HookCounts {
	added: usize,
	removed: usize,
}

fn caption_added(
	mut world: crate::bevy::ecs::world::DeferredWorld,
	_: crate::compatibility::HookContext,
) {
	if let Some(mut counts) = world.get_resource_mut::<HookCounts>() {
		counts.added += 1;
	}
}

fn caption_removed(
	mut world: crate::bevy::ecs::world::DeferredWorld,
	_: crate::compatibility::HookContext,
) {
	if let Some(mut counts) = world.get_resource_mut::<HookCounts>() {
		counts.removed += 1;
	}
}

impl LocalizationTarget for HookedCaption {
	type Data = ();
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let Some(mut caption) = entity.get_mut::<Self>() else {
			let mut caption = Self::default();
			text.clone_into(&mut caption.text);
			entity.insert(caption);
			return;
		};

		if caption.text == text {
			return;
		}

		text.clone_into(&mut caption.text);
	}
}

#[derive(Component, Default)]
struct SemanticCaption(String);

impl LocalizationTarget for SemanticCaption {
	type Data = ();
	type Filter = ();

	const PHASE: crate::TargetUpdatePhase = crate::TargetUpdatePhase::AfterLayout;

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let Some(mut caption) = entity.get_mut::<Self>() else {
			let mut caption = Self::default();
			text.clone_into(&mut caption.0);
			entity.insert(caption);
			return;
		};

		if caption.0 == text {
			return;
		}

		text.clone_into(&mut caption.0);
	}
}

#[test]
fn semantic_bindings_added_between_refresh_phases_use_the_ready_catalog() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin()));
	let owner = app.world_mut().spawn(binding()).id();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	app.add_systems(
		PostUpdate,
		(|mut commands: Commands, mut spawned: Local<bool>| {
			if *spawned {
				return;
			}

			commands.spawn(Localized::<SemanticCaption, TestCatalog>::new(|c| {
				c.0.clone()
			}));
			*spawned = true;
		})
		.after(crate::LocalizationSystems::Refresh)
		.before(crate::LocalizationAccessibilitySet),
	);
	app.update();
	let entity = app
		.world_mut()
		.query_filtered::<Entity, With<SemanticCaption>>()
		.single(app.world())
		.unwrap();
	assert_eq!(
		app.world().get::<SemanticCaption>(entity).unwrap().0,
		"Japanese"
	);
	app.world_mut().despawn(owner);
	app.world_mut().despawn(entity);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn existing_localization_system_sets_remain_exhaustively_matchable() {
	use crate::LocalizationSystems;

	fn consumer(set: LocalizationSystems) -> &'static str {
		match set {
			LocalizationSystems::Publish => "publish",
			LocalizationSystems::Refresh => "refresh",
			LocalizationSystems::Progress => "progress",
		}
	}

	assert_eq!(consumer(LocalizationSystems::Publish), "publish");
	assert_eq!(consumer(LocalizationSystems::Refresh), "refresh");
	assert_eq!(consumer(LocalizationSystems::Progress), "progress");
}

fn plugin() -> LocalizationPlugin<TestCatalog> {
	LocalizationPlugin::from_bytes([
		("ja", "ui.ftl", b"Japanese".as_slice()),
		("es", "ui.ftl", b"Spanish".as_slice()),
	])
	.unwrap()
}

fn binding() -> Localized<Caption, TestCatalog> {
	Localized::<Caption, TestCatalog>::new(|catalog| catalog.0.clone())
}

#[test]
fn custom_bindings_share_messages_and_adopt_targets_in_either_provider_order() {
	for provider_first in [true, false] {
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = calls.clone();
		let message = Msg::<TestCatalog>::new(move |catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			catalog.0.clone()
		});
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init_resource::<HookCounts>();

		if provider_first {
			app.add_plugins(plugin());
		}

		let owner = app
			.world_mut()
			.spawn((
				HookedCaption::default(),
				Localized::<HookedCaption, TestCatalog>::from(message.clone()),
				SemanticCaption::default(),
				Localized::<SemanticCaption, TestCatalog>::from(message),
			))
			.id();

		if !provider_first {
			app.add_plugins(plugin());
		}

		pump(&mut app, |world| {
			world.get::<HookedCaption>(owner).unwrap().text == "Japanese"
				&& world.get::<SemanticCaption>(owner).unwrap().0 == "Japanese"
		});
		assert_eq!(calls.load(Ordering::Relaxed), 2);

		for _ in 0..4 {
			app.update();
		}

		assert_eq!(calls.load(Ordering::Relaxed), 2);
		app.world_mut()
			.resource_mut::<Localization<TestCatalog>>()
			.set_locale("es");
		pump(&mut app, |world| {
			world.get::<HookedCaption>(owner).unwrap().text == "Spanish"
				&& world.get::<SemanticCaption>(owner).unwrap().0 == "Spanish"
		});
		assert_eq!(calls.load(Ordering::Relaxed), 4);
		app.world_mut()
			.entity_mut(owner)
			.remove::<Localized<HookedCaption, TestCatalog>>();
		app.update();
		assert!(app.world().contains_resource::<TestCatalog>());
		assert_eq!(
			app.world().get::<HookedCaption>(owner).unwrap().text,
			"Spanish"
		);
		app.world_mut().despawn(owner);
		app.update();
		assert!(!app.world().contains_resource::<TestCatalog>());
		assert_eq!(app.world().resource::<HookCounts>().added, 1);
		assert_eq!(app.world().resource::<HookCounts>().removed, 1);
	}
}

#[test]
fn unbound_custom_targets_and_messages_do_not_request_a_catalog() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin()));
	let _message = Msg::<TestCatalog>::new(|c| c.0.clone());
	let target = app
		.world_mut()
		.spawn(HookedCaption {
			text: "owned".into(),
		})
		.id();

	for _ in 0..4 {
		app.update();
	}

	assert!(!app.world().contains_resource::<TestCatalog>());
	assert!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.desired()
			.is_empty()
	);
	assert_eq!(
		app.world().get::<HookedCaption>(target).unwrap().text,
		"owned"
	);
	app.world_mut().spawn(binding());
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert_eq!(
		app.world().get::<HookedCaption>(target).unwrap().text,
		"owned"
	);
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn disabled_native_and_custom_bindings_keep_demand_and_resume_together() {
	use crate::bevy::ecs::entity_disabling::Disabled;

	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin()));
	let owner = app
		.world_mut()
		.spawn((
			binding(),
			Localized::<Text, TestCatalog>::new(|c| c.0.clone()),
		))
		.id();
	pump(&mut app, |world| {
		world
			.get::<Caption>(owner)
			.is_some_and(|c| c.0 == "Japanese")
	});
	app.world_mut().entity_mut(owner).insert(Disabled);
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |world| {
		world
			.get_resource::<TestCatalog>()
			.is_some_and(|c| c.0 == "Spanish")
	});
	assert_eq!(app.world().get::<Caption>(owner).unwrap().0, "Japanese");
	assert_eq!(app.world().get::<Text>(owner).unwrap().0, "Japanese");
	app.world_mut().entity_mut(owner).remove::<Disabled>();
	app.update();
	assert_eq!(app.world().get::<Caption>(owner).unwrap().0, "Spanish");
	assert_eq!(app.world().get::<Text>(owner).unwrap().0, "Spanish");
	app.world_mut().despawn(owner);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn custom_and_native_bindings_release_their_scope_only_after_the_last_owner() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin()));
	let _passive = binding();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	let custom = app.world_mut().spawn(binding()).id();
	let native = app
		.world_mut()
		.spawn(Localized::<Text, TestCatalog>::new(|c| c.0.clone()))
		.id();
	pump(&mut app, |world| {
		world
			.get::<Caption>(custom)
			.is_some_and(|c| c.0 == "Japanese")
	});
	assert!(app.world().get::<Text>(custom).is_none());
	app.world_mut().despawn(custom);
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
	app.world_mut().despawn(native);
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	let later = app.world_mut().spawn(binding()).id();
	pump(&mut app, |world| {
		world
			.get::<Caption>(later)
			.is_some_and(|c| c.0 == "Japanese")
	});
}

#[test]
fn pre_plugin_companion_adoption_cancellation_and_replacement_are_balanced() {
	let mut app = App::new();
	let retained = app.world_mut().spawn(binding()).id();
	let cancelled = app.world_mut().spawn(binding()).id();
	app.world_mut().despawn(cancelled);
	app.world_mut().flush();
	app.add_plugins((MinimalPlugins, plugin()));
	pump(&mut app, |world| {
		world
			.get::<Caption>(retained)
			.is_some_and(|c| c.0 == "Japanese")
	});
	app.world_mut()
		.entity_mut(retained)
		.insert(Localized::<Caption, TestCatalog>::new(|c| {
			format!("{}!", c.0)
		}));
	app.update();
	assert_eq!(app.world().get::<Caption>(retained).unwrap().0, "Japanese!");
	app.world_mut()
		.entity_mut(retained)
		.remove::<Localized<Caption, TestCatalog>>();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
}

#[test]
fn generic_targets_format_only_on_input_changes_and_repair_readded_targets() {
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin()));
	let entity = app
		.world_mut()
		.spawn(Localized::<Caption, TestCatalog>::new(move |c| {
			observed.fetch_add(1, Ordering::Relaxed);
			c.0.clone()
		}))
		.id();
	pump(&mut app, |world| {
		world
			.get::<Caption>(entity)
			.is_some_and(|c| c.0 == "Japanese")
	});
	let before = calls.load(Ordering::Relaxed);
	let tick = app
		.world()
		.entity(entity)
		.get_ref::<Caption>()
		.unwrap()
		.last_changed();

	for _ in 0..4 {
		app.update();
	}

	assert_eq!(calls.load(Ordering::Relaxed), before);
	assert_eq!(
		app.world()
			.entity(entity)
			.get_ref::<Caption>()
			.unwrap()
			.last_changed(),
		tick
	);
	app.world_mut().entity_mut(entity).remove::<Caption>();
	app.update();
	assert_eq!(app.world().get::<Caption>(entity).unwrap().0, "Japanese");
	assert_eq!(calls.load(Ordering::Relaxed), before);
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |world| {
		world
			.get::<Caption>(entity)
			.is_some_and(|c| c.0 == "Spanish")
	});
	assert!(calls.load(Ordering::Relaxed) > before);
}

#[test]
fn custom_target_hooks_survive_binding_replacement_and_cached_repairs() {
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let mut app = App::new();
	app.init_resource::<HookCounts>();
	let target = app
		.world_mut()
		.spawn((
			HookedCaption::default(),
			Localized::<HookedCaption, TestCatalog>::new(move |c| {
				observed.fetch_add(1, Ordering::Relaxed);
				c.0.clone()
			}),
		))
		.id();
	app.add_plugins((MinimalPlugins, plugin()));
	pump(&mut app, |world| {
		world.get::<HookedCaption>(target).unwrap().text == "Japanese"
	});
	let before = calls.load(Ordering::Relaxed);
	let tick = app
		.world()
		.entity(target)
		.get_ref::<HookedCaption>()
		.unwrap()
		.last_changed();

	for _ in 0..4 {
		app.update();
	}

	assert_eq!(calls.load(Ordering::Relaxed), before);
	assert_eq!(
		app.world()
			.entity(target)
			.get_ref::<HookedCaption>()
			.unwrap()
			.last_changed(),
		tick
	);
	app.world_mut()
		.get_mut::<HookedCaption>(target)
		.unwrap()
		.text = "edited".into();
	app.update();
	assert_eq!(
		app.world().get::<HookedCaption>(target).unwrap().text,
		"Japanese"
	);
	assert_eq!(calls.load(Ordering::Relaxed), before);
	app.world_mut()
		.entity_mut(target)
		.insert(Localized::<HookedCaption, TestCatalog>::new(|c| {
			format!("{}!", c.0)
		}));
	app.update();
	assert_eq!(
		app.world().get::<HookedCaption>(target).unwrap().text,
		"Japanese!"
	);
	assert_eq!(app.world().resource::<HookCounts>().added, 1);
	assert_eq!(app.world().resource::<HookCounts>().removed, 0);
	app.world_mut().entity_mut(target).remove::<HookedCaption>();
	app.update();
	assert_eq!(
		app.world().get::<HookedCaption>(target).unwrap().text,
		"Japanese!"
	);
	assert_eq!(app.world().resource::<HookCounts>().added, 2);
	assert_eq!(app.world().resource::<HookCounts>().removed, 1);
	app.world_mut()
		.entity_mut(target)
		.remove::<Localized<HookedCaption, TestCatalog>>();
	app.update();
	assert!(!app.world().contains_resource::<TestCatalog>());
	assert_eq!(
		app.world().get::<HookedCaption>(target).unwrap().text,
		"Japanese!"
	);
	assert_eq!(app.world().resource::<HookCounts>().removed, 1);
	app.world_mut().despawn(target);
	assert_eq!(app.world().resource::<HookCounts>().removed, 2);
}

#[test]
fn pending_custom_consumers_cancel_on_last_removal_and_failed_switches_keep_active_values() {
	let reads = Arc::new(AtomicUsize::new(0));
	let observed = reads.clone();
	let pending = LocalizationPlugin::<TestCatalog>::from_loader(move |_, _| {
		observed.fetch_add(1, Ordering::Relaxed);
		std::future::pending::<Result<Vec<u8>, String>>()
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, pending));
	let entity = app
		.world_mut()
		.spawn(Localized::<HookedCaption, TestCatalog>::new(|c| {
			c.0.clone()
		}))
		.id();
	pump(&mut app, |_| reads.load(Ordering::Relaxed) == 1);
	app.world_mut().despawn(entity);
	app.update();
	assert_eq!(
		app.world().resource::<Localization<TestCatalog>>().pending,
		0
	);
	assert!(
		app.world()
			.resource::<Localization<TestCatalog>>()
			.desired()
			.is_empty()
	);

	let plugin = LocalizationPlugin::<TestCatalog>::from_loader(|locale, _| {
		std::future::ready(if locale == "es" {
			Err("unavailable")
		} else {
			Ok(b"Japanese".to_vec())
		})
	});
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin));
	let custom = app.world_mut().spawn(binding()).id();
	let stored = app
		.world_mut()
		.spawn(Localized::<HookedCaption, TestCatalog>::new(|c| {
			c.0.clone()
		}))
		.id();
	pump(&mut app, |world| {
		world.get::<HookedCaption>(stored).unwrap().text == "Japanese"
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog>>()
		.set_locale("es");
	pump(&mut app, |world| {
		matches!(
			world
				.resource::<Localization<TestCatalog>>()
				.preparation_status(),
			crate::PreparationStatus::Failed(_)
		)
	});
	assert_eq!(app.world().get::<Caption>(custom).unwrap().0, "Japanese");
	assert_eq!(
		app.world().get::<HookedCaption>(stored).unwrap().text,
		"Japanese"
	);
}

#[test]
fn full_keeps_its_eager_loading_policy_without_binding_owners() {
	let mut app = App::new();
	let plugin = LocalizationPlugin::<TestCatalog, crate::Full>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap();
	app.add_plugins((MinimalPlugins, plugin));
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	let entity = app.world_mut().spawn(binding()).id();
	app.update();
	assert_eq!(app.world().get::<Caption>(entity).unwrap().0, "Japanese");
	app.world_mut().despawn(entity);
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
#[allow(
	deprecated,
	reason = "Exercise the older Button accessibility producer retained by these releases"
)]
fn semantic_binding_leaves_other_native_ui_producers_in_control_of_their_nodes() {
	use crate::bevy::{
		a11y::AccessibilityNode, input::InputPlugin, text::TextPlugin, ui::UiPlugin,
	};

	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			InputPlugin,
			TextPlugin,
			UiPlugin,
			plugin().with_cache_policy(policy),
		));
		app.init_asset::<Image>();
		app.init_asset::<TextureAtlasLayout>();
		let entity = app
			.world_mut()
			.spawn((
				ImageNode::default(),
				Localized::<AccessibleLabel, TestCatalog>::new(|c| c.0.clone()),
			))
			.id();
		pump(&mut app, |world| {
			world
				.get::<AccessibilityNode>(entity)
				.is_some_and(|node| node.label() == Some("Japanese"))
		});
		app.world_mut()
			.entity_mut(entity)
			.insert(ImageNode::default());
		app.update();
		assert_eq!(
			app.world()
				.get::<AccessibilityNode>(entity)
				.unwrap()
				.label(),
			None
		);
		assert_eq!(
			app.world().get::<AccessibleLabel>(entity).unwrap().0,
			"Japanese"
		);
	}
}

#[derive(Component)]
struct Card {
	title: String,
	body: String,
}

struct CardTitle;

struct CardBody;

impl LocalizationTarget for CardTitle {
	type Data = ();
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let Some(mut card) = entity.get_mut::<Card>() else {
			return;
		};

		if card.title == text {
			return;
		}

		text.clone_into(&mut card.title);
	}
}

impl LocalizationTarget for CardBody {
	type Data = ();
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let Some(mut card) = entity.get_mut::<Card>() else {
			return;
		};

		if card.body == text {
			return;
		}

		text.clone_into(&mut card.body);
	}
}

#[test]
fn field_adapters_and_explicit_text_targets_remain_independent() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, plugin()));
	let entity = app
		.world_mut()
		.spawn((
			Localized::<CardTitle, TestCatalog>::new(|c| format!("title:{}", c.0)),
			Localized::<CardBody, TestCatalog>::new(|c| format!("body:{}", c.0)),
		))
		.id();
	pump(&mut app, |world| world.contains_resource::<TestCatalog>());
	assert!(app.world().get::<Card>(entity).is_none());
	app.world_mut().entity_mut(entity).insert(Card {
		title: String::new(),
		body: String::new(),
	});
	app.update();
	let world_text = app
		.world_mut()
		.spawn(Localized::<Text2d, TestCatalog>::new(|c| c.0.clone()))
		.id();
	let span = app
		.world_mut()
		.spawn(Localized::<TextSpan, TestCatalog>::new(|c| c.0.clone()))
		.id();
	pump(&mut app, |world| {
		world.get::<Card>(entity).unwrap().body == "body:Japanese"
	});
	assert_eq!(
		app.world().get::<Card>(entity).unwrap().title,
		"title:Japanese"
	);
	assert_eq!(app.world().get::<Text2d>(world_text).unwrap().0, "Japanese");
	assert_eq!(app.world().get::<TextSpan>(span).unwrap().0, "Japanese");
	assert!(app.world().get::<Text>(world_text).is_none());
	assert!(app.world().get::<Text>(span).is_none());
	app.world_mut()
		.entity_mut(entity)
		.remove::<Localized<CardTitle, TestCatalog>>();
	app.update();
	assert!(app.world().contains_resource::<TestCatalog>());
}

#[test]
fn manual_targets_clear_without_loading_and_follow_explicit_requests() {
	let mut app = App::new();
	let plugin = LocalizationPlugin::<TestCatalog, Manual>::from_bytes([(
		"ja",
		"ui.ftl",
		b"Japanese".as_slice(),
	)])
	.unwrap();
	app.add_plugins((MinimalPlugins, plugin));
	let custom = app
		.world_mut()
		.spawn((Caption("stale".into()), binding()))
		.id();
	let stored = app
		.world_mut()
		.spawn((
			HookedCaption {
				text: "stale".into(),
			},
			Localized::<HookedCaption, TestCatalog>::new(|c| c.0.clone()),
		))
		.id();
	app.update();
	assert!(app.world().get::<Caption>(custom).unwrap().0.is_empty());
	assert!(
		app.world()
			.get::<HookedCaption>(stored)
			.unwrap()
			.text
			.is_empty()
	);
	assert!(!app.world().contains_resource::<TestCatalog>());
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.load::<TestCatalog>();
	pump(&mut app, |world| {
		world.get::<HookedCaption>(stored).unwrap().text == "Japanese"
	});
	app.world_mut()
		.resource_mut::<Localization<TestCatalog, Manual>>()
		.unload::<TestCatalog>();
	app.update();
	assert!(app.world().get::<Caption>(custom).unwrap().0.is_empty());
	assert!(
		app.world()
			.get::<HookedCaption>(stored)
			.unwrap()
			.text
			.is_empty()
	);
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn accessibility_bindings_do_not_repair_names_written_by_other_node_owners() {
	use crate::bevy::a11y::AccessibilityNode;

	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let calls = Arc::new(AtomicUsize::new(0));
		let observed = calls.clone();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, plugin().with_cache_policy(policy)));
		app.add_systems(
			PostUpdate,
			(|mut nodes: Query<&mut AccessibilityNode>| {
				for mut node in &mut nodes {
					node.set_label("engine-produced label");
				}
			})
			.in_set(crate::compatibility::UiSystems::PostLayout),
		);
		let entity = app
			.world_mut()
			.spawn(Localized::<AccessibleLabel, TestCatalog>::new(move |c| {
				observed.fetch_add(1, Ordering::Relaxed);
				c.0.clone()
			}))
			.id();
		pump(&mut app, |world| {
			world
				.get::<AccessibleLabel>(entity)
				.is_some_and(|label| label.0 == "Japanese")
		});
		let before = calls.load(Ordering::Relaxed);

		for _ in 0..4 {
			app.update();
			assert_eq!(
				app.world()
					.get::<AccessibilityNode>(entity)
					.unwrap()
					.label(),
				Some("engine-produced label")
			);
		}

		assert_eq!(calls.load(Ordering::Relaxed), before);
		assert!(app.world().get::<Text>(entity).is_none());
		app.world_mut()
			.resource_mut::<Localization<TestCatalog>>()
			.set_locale("es");
		pump(&mut app, |world| {
			world.get::<AccessibleLabel>(entity).unwrap().0 == "Spanish"
		});
		assert_eq!(
			app.world()
				.get::<AccessibilityNode>(entity)
				.unwrap()
				.label(),
			Some("Spanish")
		);
		app.world_mut().despawn(entity);
		app.update();
		assert!(!app.world().contains_resource::<TestCatalog>());
	}
}
