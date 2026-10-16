use super::tests::Catalog;
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{
		App, Component, DetectChanges, Entity, EntityWorldMut, MinimalPlugins, Or, Resource, With,
		Without, World,
	},
};
use crate::bindings::Binding;
use crate::cache_policy::ProviderCachePolicy;
use crate::{CachePolicy, LocalizationTarget, LocalizationUpdate, Localized, Msg, TargetItem};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Component, Default)]
struct Caption(String);

#[derive(Component, Default)]
struct Mirror(String);

#[derive(Component)]
struct Enabled;

#[derive(Component)]
struct Paused;

#[derive(Component)]
struct Menu;

#[derive(Component)]
struct Overlay;

struct Paired;

impl LocalizationTarget for Paired {
	type Data = (Option<&'static mut Caption>, Option<&'static mut Mirror>);
	type Filter = (
		With<Enabled>,
		Without<Paused>,
		Or<(With<Menu>, With<Overlay>)>,
	);

	fn update(
		(caption, mirror): TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		let (Some(mut caption), Some(mut mirror)) = (caption, mirror) else {
			return crate::UpdateResult::Deferred;
		};

		if !update.is_changed() && !caption.is_changed() {
			if mirror.0 != caption.0 {
				caption.0.clone_into(&mut mirror.0);
			}

			return crate::UpdateResult::Done;
		}

		let text = update.text();

		if caption.0 != text {
			text.clone_into(&mut caption.0);
		}

		if mirror.0 != text {
			text.clone_into(&mut mirror.0);
		}

		// Reading the value twice must not invoke an uncached formatter twice.
		assert_eq!(update.text(), caption.0);
		crate::UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		if let Some(mut caption) = entity.get_mut::<Caption>() {
			if caption.0 != text {
				text.clone_into(&mut caption.0);
			}
		} else {
			let mut caption = Caption::default();
			text.clone_into(&mut caption.0);
			entity.insert(caption);
		}

		if let Some(mut mirror) = entity.get_mut::<Mirror>() {
			if mirror.0 != text {
				text.clone_into(&mut mirror.0);
			}
		} else {
			let mut mirror = Mirror::default();
			text.clone_into(&mut mirror.0);
			entity.insert(mirror);
		}
	}
}

fn app(policy: CachePolicy) -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.world_mut()
		.insert_resource(ProviderCachePolicy::<Catalog>::new(policy));
	app
}

fn message(calls: &Arc<AtomicUsize>) -> Msg<Catalog> {
	let calls = calls.clone();
	Msg::new(move |catalog: &Catalog| {
		calls.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	})
}

fn refresh<T: LocalizationTarget>(world: &mut World) {
	world.flush();
	<Localized<T, Catalog> as Binding>::refresh(world);
}

#[test]
fn typed_pairs_use_native_filters_and_lazy_message_reads() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		for count in [1, 4 << 10] {
			let mut app = app(policy);
			let world = app.world_mut();
			world.insert_resource(Catalog("first".into()));
			let calls = Arc::new(AtomicUsize::new(0));
			let message = message(&calls);
			let owners: Vec<_> = world
				.spawn_batch((0..count).map(|_| {
					(
						Enabled,
						Menu,
						Caption::default(),
						Mirror::default(),
						Localized::<Paired, Catalog>::from(message.clone()),
					)
				}))
				.collect();
			let excluded = world
				.spawn((
					Enabled,
					Overlay,
					Paused,
					Localized::<Paired, Catalog>::from(message.clone()),
				))
				.id();
			refresh::<Paired>(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			refresh::<Paired>(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			assert!(!world.entity(excluded).contains::<Caption>());

			world.increment_change_tick();
			world.get_mut::<Mirror>(owners[0]).unwrap().0 = "external".into();
			refresh::<Paired>(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			assert_eq!(world.get::<Mirror>(owners[0]).unwrap().0, "first");
			world.increment_change_tick();
			world.get_mut::<Caption>(owners[0]).unwrap().0 = "external".into();
			refresh::<Paired>(world);
			let repairs = usize::from(matches!(policy, CachePolicy::None));
			assert_eq!(calls.load(Ordering::Relaxed), count + repairs);
			assert_eq!(world.get::<Caption>(owners[0]).unwrap().0, "first");

			world.entity_mut(owners[0]).insert(Paused);
			world.insert_resource(Catalog("second".into()));
			refresh::<Paired>(world);
			assert_eq!(world.get::<Caption>(owners[0]).unwrap().0, "first");
			world.entity_mut(owners[0]).remove::<Paused>();
			world.entity_mut(excluded).remove::<Paused>();
			refresh::<Paired>(world);
			assert_eq!(world.get::<Caption>(owners[0]).unwrap().0, "second");
			assert_eq!(world.get::<Caption>(excluded).unwrap().0, "second");

			world.entity_mut(owners[0]).remove::<Menu>();
			world.insert_resource(Catalog("third".into()));
			refresh::<Paired>(world);
			assert_eq!(world.get::<Caption>(owners[0]).unwrap().0, "second");
			world.entity_mut(owners[0]).insert(Overlay);
			refresh::<Paired>(world);
			assert_eq!(world.get::<Mirror>(owners[0]).unwrap().0, "third");
			world.entity_mut(owners[0]).remove::<Enabled>();
			world.insert_resource(Catalog("fourth".into()));
			refresh::<Paired>(world);
			assert_eq!(world.get::<Caption>(owners[0]).unwrap().0, "third");
			world.entity_mut(owners[0]).insert(Enabled);
			refresh::<Paired>(world);
			assert_eq!(world.get::<Caption>(owners[0]).unwrap().0, "fourth");

			world
				.entity_mut(owners[0])
				.insert(Localized::<Paired, Catalog>::new(|_| "replacement".into()));
			refresh::<Paired>(world);
			assert_eq!(world.get::<Mirror>(owners[0]).unwrap().0, "replacement");
			world.entity_mut(owners[0]).remove::<Mirror>();
			refresh::<Paired>(world);
			assert_eq!(world.get::<Mirror>(owners[0]).unwrap().0, "replacement");
			world.remove_resource::<Catalog>();
			refresh::<Paired>(world);
			assert!(world.get::<Caption>(owners[0]).unwrap().0.is_empty());
			<Localized<Paired, Catalog> as Binding>::unregister(world);
		}
	}
}

struct Required;

impl LocalizationTarget for Required {
	type Data = &'static mut Caption;
	type Filter = ();

	fn update(
		mut caption: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		let text = update.text();

		if caption.0 != text {
			text.clone_into(&mut caption.0);
		}

		crate::UpdateResult::Done
	}

	fn apply(_: &mut EntityWorldMut<'_>, _update: &mut crate::LocalizationUpdate<'_>) {
		panic!("required data never requests missing-target creation");
	}
}

#[test]
fn required_data_waits_for_insertion_and_reenters_after_removal() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = app(policy);
		let world = app.world_mut();
		world.insert_resource(Catalog("first".into()));
		let calls = Arc::new(AtomicUsize::new(0));
		let owner = world
			.spawn(Localized::<Required, Catalog>::from(message(&calls)))
			.id();
		refresh::<Required>(world);
		assert_eq!(calls.load(Ordering::Relaxed), 0);
		world.entity_mut(owner).insert(Caption::default());
		refresh::<Required>(world);
		assert_eq!(world.get::<Caption>(owner).unwrap().0, "first");
		world.entity_mut(owner).remove::<Caption>();
		world.insert_resource(Catalog("second".into()));
		refresh::<Required>(world);
		world.entity_mut(owner).insert(Caption::default());
		refresh::<Required>(world);
		assert_eq!(world.get::<Caption>(owner).unwrap().0, "second");
	}
}

struct Waiting;

impl LocalizationTarget for Waiting {
	type Data = Option<&'static mut Caption>;
	type Filter = ();

	fn update(
		caption: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		let Some(mut caption) = caption else {
			return crate::UpdateResult::Done;
		};

		assert!(update.is_changed());
		update.text().clone_into(&mut caption.0);
		crate::UpdateResult::Done
	}

	fn apply(_: &mut EntityWorldMut<'_>, _update: &mut crate::LocalizationUpdate<'_>) {
		panic!("wait for application-owned target");
	}
}

#[test]
fn unread_uncached_invalidations_remain_pending_without_formatting() {
	let mut app = app(CachePolicy::None);
	let world = app.world_mut();
	let calls = Arc::new(AtomicUsize::new(0));
	let owner = world
		.spawn(Localized::<Waiting, Catalog>::from(message(&calls)))
		.id();
	refresh::<Waiting>(world);
	world.insert_resource(Catalog("first".into()));
	refresh::<Waiting>(world);
	world.insert_resource(Catalog("second".into()));
	refresh::<Waiting>(world);
	refresh::<Waiting>(world);
	assert_eq!(calls.load(Ordering::Relaxed), 0);
	world.entity_mut(owner).insert(Caption::default());
	refresh::<Waiting>(world);
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	assert_eq!(world.get::<Caption>(owner).unwrap().0, "second");
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
struct ResourceTarget;

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
impl LocalizationTarget for ResourceTarget {
	type Data = (&'static mut Catalog, &'static mut Caption);
	type Filter = ();

	fn update(
		(catalog, mut caption): TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		assert_eq!(catalog.0, update.text());
		update.text().clone_into(&mut caption.0);
		crate::UpdateResult::Done
	}

	fn apply(_: &mut EntityWorldMut<'_>, _update: &mut crate::LocalizationUpdate<'_>) {
		panic!("resource-entity target is supplied");
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn typed_data_can_borrow_the_scope_resource_entity_after_snapshotting() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = app(policy);
		let world = app.world_mut();
		world.insert_resource(Catalog("ready".into()));
		let owner = world
			.resource_entities()
			.get(world.component_id::<Catalog>().unwrap())
			.unwrap();
		world.entity_mut(owner).insert((
			Caption::default(),
			Localized::<ResourceTarget, Catalog>::new(|catalog| catalog.0.clone()),
		));
		refresh::<ResourceTarget>(world);
		assert_eq!(world.get::<Caption>(owner).unwrap().0, "ready");
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
struct Semantic;

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
impl LocalizationTarget for Semantic {
	type Data = (
		Option<crate::bevy::prelude::Ref<'static, crate::bevy::prelude::AccessibleLabel>>,
		Option<&'static mut crate::bevy::a11y::AccessibilityNode>,
	);
	type Filter = ();

	fn update(
		(label, node): TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		let (Some(label), Some(mut node)) = (label, node) else {
			return crate::UpdateResult::Deferred;
		};

		let text = if update.is_changed() || label.is_changed() {
			update.text()
		} else {
			&label.0
		};

		if label.0 != text {
			return crate::UpdateResult::Deferred;
		}

		if node.label() != Some(text) {
			node.set_label(text);
		}

		crate::UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let mut label = crate::bevy::prelude::AccessibleLabel::default();
		text.clone_into(&mut label.0);
		entity.insert(label);
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn a_user_marker_can_pair_an_immutable_foreign_label_with_its_mutable_node() {
	use crate::bevy::{a11y::AccessibilityNode, prelude::AccessibleLabel};

	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		for count in [1, 4 << 10] {
			let mut app = app(policy);
			let world = app.world_mut();
			world.insert_resource(Catalog("translated".into()));
			let calls = Arc::new(AtomicUsize::new(0));
			let message = message(&calls);
			let owners: Vec<_> = world
				.spawn_batch(
					(0..count).map(|_| Localized::<Semantic, Catalog>::from(message.clone())),
				)
				.collect();
			refresh::<Semantic>(world);
			refresh::<Semantic>(world);
			refresh::<Semantic>(world);
			let stable = calls.load(Ordering::Relaxed);

			for &owner in &owners {
				world
					.get_mut::<AccessibilityNode>(owner)
					.unwrap()
					.set_label("external");
			}

			refresh::<Semantic>(world);
			assert_eq!(calls.load(Ordering::Relaxed), stable);

			for &owner in &owners {
				assert_eq!(world.get::<AccessibleLabel>(owner).unwrap().0, "translated");
				assert_eq!(
					world.get::<AccessibilityNode>(owner).unwrap().label(),
					Some("translated")
				);
			}

			world.insert_resource(Catalog("next".into()));
			refresh::<Semantic>(world);
			assert_eq!(world.get::<AccessibleLabel>(owners[0]).unwrap().0, "next");
		}
	}
}

#[derive(Component)]
struct Skip;

struct DeferredTarget;

#[derive(Resource, Clone, Copy)]
enum Invalidate {
	Filter,
	Message,
	Despawn,
}

impl LocalizationTarget for DeferredTarget {
	type Data = Option<&'static Caption>;
	type Filter = Without<Skip>;

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut crate::LocalizationUpdate<'_>) {
		let text = update.text();

		let current = entity.id();
		entity.world_scope(|world| {
			let action = *world.resource::<Invalidate>();
			let others: Vec<_> = world
				.query_filtered::<Entity, With<Localized<Self, Catalog>>>()
				.iter(world)
				.filter(|other| *other != current)
				.collect();

			for other in others {
				match action {
					Invalidate::Filter => {
						world.entity_mut(other).insert(Skip);
					}
					Invalidate::Message => {
						world
							.entity_mut(other)
							.insert(Localized::<Self, Catalog>::new(|_| "new".into()));
					}
					Invalidate::Despawn => {
						world.despawn(other);
					}
				}
			}
		});
		let mut caption = Caption::default();
		text.clone_into(&mut caption.0);
		entity.insert(caption);
	}
}

#[test]
fn deferred_batches_recheck_filters_identity_and_despawns_after_prior_callbacks() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		for action in [Invalidate::Filter, Invalidate::Message, Invalidate::Despawn] {
			let mut app = app(policy);
			let world = app.world_mut();
			world.insert_resource(action);
			world.insert_resource(Catalog("old".into()));
			world.spawn_batch(
				(0..3).map(|_| Localized::<DeferredTarget, Catalog>::new(|c| c.0.clone())),
			);
			refresh::<DeferredTarget>(world);
			let applied = world.query::<&Caption>().iter(world).count();
			assert_eq!(
				applied, 1,
				"an earlier callback invalidated the remaining work"
			);
		}
	}
}
