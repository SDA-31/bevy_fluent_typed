//! Adapter observations follow Bevy's component tick aging without acknowledging other writes.
use super::state::State;
use crate::bevy::{
	ecs::{self as bevy_ecs, change_detection::MAX_CHANGE_AGE},
	prelude::{Component, DetectChanges, DetectChangesMut, Entity, EntityWorldMut, Mut, World},
};
use crate::bindings::Binding;
use crate::cache_policy::ProviderCachePolicy;
use crate::compatibility::Tick;
use crate::state::LocaleSnapshots;
use crate::tests::TestCatalog;
use crate::{
	CachePolicy, LocalizationTarget, LocalizationUpdate, Localized, TargetItem, TargetReadItem,
	UpdateResult,
};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Component, Default)]
struct Caption<const DIRECT: bool>(String);

impl<const DIRECT: bool> LocalizationTarget for Caption<DIRECT> {
	type Data = Mut<'static, Self>;
	type Filter = ();

	fn needs_update(data: &TargetItem<'_, '_, Self>, update: &LocalizationUpdate<'_>) -> bool {
		update.is_changed() || update.component_changed(data)
	}

	fn update(
		target: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		if !DIRECT {
			return UpdateResult::Deferred;
		}

		update.set_text(target.map_unchanged(|target| &mut target.0));
		UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get_mut::<Self>() else {
			return;
		};

		if !update.is_changed() && !update.component_changed(&target) {
			return;
		}

		update.set_text(target.map_unchanged(|target| &mut target.0));
	}

	fn matches(target: TargetReadItem<'_, '_, Self>, text: &str) -> bool {
		target.0 == text
	}
}

fn scene<const DIRECT: bool>(policy: CachePolicy) -> (World, Entity, Arc<AtomicUsize>) {
	let mut world = World::new();
	world.insert_resource(ProviderCachePolicy::<TestCatalog>::new(policy));
	let scope: crate::catalog::SharedScope = Arc::new(TestCatalog("translated".into()));
	LocaleSnapshots::<TestCatalog>::publish(&mut world, "es", Some(&scope));
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let owner = world
		.spawn((
			Caption::<DIRECT>::default(),
			Localized::<Caption<DIRECT>, TestCatalog>::new(move |scope| {
				observed.fetch_add(1, Ordering::Relaxed);
				scope.0.clone()
			})
			.with_locale("es"),
		))
		.id();
	world.flush();
	<Localized<Caption<DIRECT>, TestCatalog> as Binding>::refresh(&mut world);
	assert_eq!(world.get::<Caption<DIRECT>>(owner).unwrap().0, "translated");
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	(world, owner, calls)
}

fn repair_after_tick_aging<const DIRECT: bool>(policy: CachePolicy) {
	let (mut world, owner, calls) = scene::<DIRECT>(policy);
	let observed = world
		.entity(owner)
		.get_ref::<State<TestCatalog, Caption<DIRECT>>>()
		.unwrap()
		.last_changed()
		.get();
	let now = world.change_tick().get();
	// Emulate the component tick produced by Bevy's periodic aging after a full
	// counter cycle, without advancing the test World billions of times.
	world
		.get_mut::<State<TestCatalog, Caption<DIRECT>>>(owner)
		.unwrap()
		.set_last_changed(Tick::new(now.wrapping_sub(MAX_CHANGE_AGE)));
	let mut target = world.get_mut::<Caption<DIRECT>>(owner).unwrap();
	target.0 = "application edit".into();
	// This edit is newer in absolute time, but its raw u32 tick precedes the
	// original observation. An unaged observation would incorrectly skip it.
	target.set_last_changed(Tick::new(observed.wrapping_sub(1)));
	<Localized<Caption<DIRECT>, TestCatalog> as Binding>::refresh(&mut world);
	assert_eq!(world.get::<Caption<DIRECT>>(owner).unwrap().0, "translated");
	let repaired_calls = 1 + usize::from(matches!(policy, CachePolicy::None));
	assert_eq!(calls.load(Ordering::Relaxed), repaired_calls);
	<Localized<Caption<DIRECT>, TestCatalog> as Binding>::refresh(&mut world);
	assert_eq!(calls.load(Ordering::Relaxed), repaired_calls);
}

#[test]
fn fixed_bindings_repair_edits_after_observation_ticks_age_and_wrap() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		repair_after_tick_aging::<true>(policy);
		repair_after_tick_aging::<false>(policy);
	}
}

fn synchronize_without_acknowledging_edits<const DIRECT: bool>(policy: CachePolicy) {
	let (mut world, owner, calls) = scene::<DIRECT>(policy);
	let observed = world
		.entity(owner)
		.get_ref::<State<TestCatalog, Caption<DIRECT>>>()
		.unwrap()
		.last_changed();
	world.increment_change_tick();
	world.get_mut::<Caption<DIRECT>>(owner).unwrap().0 = "application edit".into();
	<Localized<Caption<DIRECT>, TestCatalog> as Binding>::synchronize_consumers(&mut world);
	assert_eq!(
		world
			.entity(owner)
			.get_ref::<State<TestCatalog, Caption<DIRECT>>>()
			.unwrap()
			.last_changed(),
		observed
	);
	<Localized<Caption<DIRECT>, TestCatalog> as Binding>::refresh(&mut world);
	assert_eq!(world.get::<Caption<DIRECT>>(owner).unwrap().0, "translated");
	assert_eq!(
		calls.load(Ordering::Relaxed),
		1 + usize::from(matches!(policy, CachePolicy::None))
	);
}

#[test]
fn demand_synchronization_does_not_acknowledge_target_edits() {
	for policy in [CachePolicy::Rendered, CachePolicy::None] {
		synchronize_without_acknowledging_edits::<true>(policy);
		synchronize_without_acknowledging_edits::<false>(policy);
	}
}
