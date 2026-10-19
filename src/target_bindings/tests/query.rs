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
struct Paired(String);

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

impl LocalizationTarget for Paired {
	type Data = (Option<&'static mut Paired>, Option<&'static mut Mirror>);
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

		if let Some(mut caption) = entity.get_mut::<Paired>() {
			if caption.0 != text {
				text.clone_into(&mut caption.0);
			}
		} else {
			let mut caption = Paired::default();
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
						Paired::default(),
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
					Paired::default(),
					Localized::<Paired, Catalog>::from(message.clone()),
				))
				.id();
			refresh::<Paired>(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			refresh::<Paired>(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			assert!(world.get::<Paired>(excluded).unwrap().0.is_empty());

			world.increment_change_tick();
			world.get_mut::<Mirror>(owners[0]).unwrap().0 = "external".into();
			refresh::<Paired>(world);
			assert_eq!(calls.load(Ordering::Relaxed), count);
			assert_eq!(world.get::<Mirror>(owners[0]).unwrap().0, "first");
			world.increment_change_tick();
			world.get_mut::<Paired>(owners[0]).unwrap().0 = "external".into();
			refresh::<Paired>(world);
			let repairs = usize::from(matches!(policy, CachePolicy::None));
			assert_eq!(calls.load(Ordering::Relaxed), count + repairs);
			assert_eq!(world.get::<Paired>(owners[0]).unwrap().0, "first");

			world.entity_mut(owners[0]).insert(Paused);
			world.insert_resource(Catalog("second".into()));
			refresh::<Paired>(world);
			assert_eq!(world.get::<Paired>(owners[0]).unwrap().0, "first");
			world.entity_mut(owners[0]).remove::<Paused>();
			world.entity_mut(excluded).remove::<Paused>();
			refresh::<Paired>(world);
			assert_eq!(world.get::<Paired>(owners[0]).unwrap().0, "second");
			assert_eq!(world.get::<Paired>(excluded).unwrap().0, "second");

			world.entity_mut(owners[0]).remove::<Menu>();
			world.insert_resource(Catalog("third".into()));
			refresh::<Paired>(world);
			assert_eq!(world.get::<Paired>(owners[0]).unwrap().0, "second");
			world.entity_mut(owners[0]).insert(Overlay);
			refresh::<Paired>(world);
			assert_eq!(world.get::<Mirror>(owners[0]).unwrap().0, "third");
			world.entity_mut(owners[0]).remove::<Enabled>();
			world.insert_resource(Catalog("fourth".into()));
			refresh::<Paired>(world);
			assert_eq!(world.get::<Paired>(owners[0]).unwrap().0, "third");
			world.entity_mut(owners[0]).insert(Enabled);
			refresh::<Paired>(world);
			assert_eq!(world.get::<Paired>(owners[0]).unwrap().0, "fourth");

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
			assert!(world.get::<Paired>(owners[0]).unwrap().0.is_empty());
			<Localized<Paired, Catalog> as Binding>::unregister(world);
		}
	}
}

#[derive(Component, Default)]
struct Required(String);

impl LocalizationTarget for Required {
	type Data = &'static mut Required;
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
fn required_data_reenters_after_explicit_target_removal() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = app(policy);
		let world = app.world_mut();
		world.insert_resource(Catalog("first".into()));
		let calls = Arc::new(AtomicUsize::new(0));
		let owner = world
			.spawn((
				Required::default(),
				Localized::<Required, Catalog>::from(message(&calls)),
			))
			.id();
		world.flush();
		world.entity_mut(owner).remove::<Required>();
		refresh::<Required>(world);
		assert_eq!(calls.load(Ordering::Relaxed), 0);
		world.entity_mut(owner).insert(Required::default());
		refresh::<Required>(world);
		assert_eq!(world.get::<Required>(owner).unwrap().0, "first");
		world.entity_mut(owner).remove::<Required>();
		world.insert_resource(Catalog("second".into()));
		refresh::<Required>(world);
		world.entity_mut(owner).insert(Required::default());
		refresh::<Required>(world);
		assert_eq!(world.get::<Required>(owner).unwrap().0, "second");
	}
}

#[derive(Component, Default)]
struct Waiting(String);

impl LocalizationTarget for Waiting {
	type Data = Option<&'static mut Waiting>;
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
		.spawn((
			Waiting::default(),
			Localized::<Waiting, Catalog>::from(message(&calls)),
		))
		.id();
	world.flush();
	world.entity_mut(owner).remove::<Waiting>();
	refresh::<Waiting>(world);
	world.insert_resource(Catalog("first".into()));
	refresh::<Waiting>(world);
	world.insert_resource(Catalog("second".into()));
	refresh::<Waiting>(world);
	refresh::<Waiting>(world);
	assert_eq!(calls.load(Ordering::Relaxed), 0);
	world.entity_mut(owner).insert(Waiting::default());
	refresh::<Waiting>(world);
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	assert_eq!(world.get::<Waiting>(owner).unwrap().0, "second");
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[derive(Component, Default)]
struct ResourceTarget(String);

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
impl LocalizationTarget for ResourceTarget {
	type Data = (&'static mut Catalog, &'static mut ResourceTarget);
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
			ResourceTarget::default(),
			Localized::<ResourceTarget, Catalog>::new(|catalog| catalog.0.clone()),
		));
		refresh::<ResourceTarget>(world);
		assert_eq!(world.get::<ResourceTarget>(owner).unwrap().0, "ready");
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[derive(Component, Default)]
struct CatalogWriter(String);

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
impl LocalizationTarget for CatalogWriter {
	type Data = (
		&'static mut Catalog,
		crate::bevy::prelude::Mut<'static, Self>,
	);
	type Filter = ();

	fn needs_update(
		(_, target): &TargetItem<'_, '_, Self>,
		update: &LocalizationUpdate<'_>,
	) -> bool {
		update.is_changed() || target.is_changed()
	}

	fn update(
		(mut catalog, target): TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> crate::UpdateResult {
		if catalog.0 == "first" {
			catalog.0 = "second".into();
		}

		update.set_text(target.map_unchanged(|target| &mut target.0));
		crate::UpdateResult::Done
	}

	fn apply(_: &mut EntityWorldMut<'_>, _: &mut LocalizationUpdate<'_>) {
		panic!("resource-entity target writes through its query");
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
#[test]
fn catalog_writes_inside_the_adapter_invalidate_the_next_refresh() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		let mut app = app(policy);
		let world = app.world_mut();
		world.insert_resource(Catalog("first".into()));
		let owner = world
			.resource_entities()
			.get(world.component_id::<Catalog>().unwrap())
			.unwrap();
		let calls = Arc::new(AtomicUsize::new(0));
		world.entity_mut(owner).insert((
			CatalogWriter::default(),
			Localized::<CatalogWriter, Catalog>::from(message(&calls)),
		));

		// Match separate publication and update ticks in ordinary frame execution.
		world.increment_change_tick();
		refresh::<CatalogWriter>(world);
		assert_eq!(world.get::<CatalogWriter>(owner).unwrap().0, "first");
		assert_eq!(world.resource::<Catalog>().0, "second");
		refresh::<CatalogWriter>(world);
		assert_eq!(world.get::<CatalogWriter>(owner).unwrap().0, "second");
		assert_eq!(calls.load(Ordering::Relaxed), 2);
		refresh::<CatalogWriter>(world);
		assert_eq!(calls.load(Ordering::Relaxed), 2);
	}
}

#[derive(Component)]
struct Skip;

#[derive(Component, Default)]
struct DeferredTarget(String);

#[derive(Resource, Clone, Copy)]
enum Invalidate {
	Filter,
	Message,
	Despawn,
}

impl LocalizationTarget for DeferredTarget {
	type Data = Option<&'static DeferredTarget>;
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
		let mut caption = DeferredTarget::default();
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
			world.spawn_batch((0..3).map(|_| {
				(
					DeferredTarget::default(),
					Localized::<DeferredTarget, Catalog>::new(|c| c.0.clone()),
				)
			}));
			refresh::<DeferredTarget>(world);
			let applied = world
				.query::<&DeferredTarget>()
				.iter(world)
				.filter(|target| target.0 == "old")
				.count();
			assert_eq!(
				applied, 1,
				"an earlier callback invalidated the remaining work"
			);
		}
	}
}
