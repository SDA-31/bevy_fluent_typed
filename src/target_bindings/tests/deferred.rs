//! Queued structural writes cannot outlive their message, language or snapshot.
use super::{Deferred, queue};
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{Component, DetectChanges, Entity, EntityWorldMut, World},
};
use crate::state::LocaleSnapshots;
use crate::tests::TestCatalog;
use crate::{LocalizationTarget, LocalizationUpdate, Localized, Msg};
use std::sync::Arc;

#[derive(Component)]
struct Caption(String);

impl LocalizationTarget for Caption {
	type Data = &'static Self;
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		entity.get_mut::<Self>().unwrap().0 = update.text().to_owned();
	}
}

fn scene(locale: Option<&'static str>) -> (World, Entity, Msg<TestCatalog>) {
	let mut world = World::new();
	world.insert_resource(TestCatalog("old".into()));
	let message = Msg::<TestCatalog>::new(|catalog| catalog.0.clone());
	let mut binding = Localized::<Caption, _>::from(message.clone());
	binding.locale = locale;
	let entity = world.spawn((Caption("untouched".into()), binding)).id();
	world.flush();
	(world, entity, message)
}

fn work(
	world: &World,
	entity: Entity,
	message: &Msg<TestCatalog>,
	locale: Option<&'static str>,
) -> Deferred<TestCatalog> {
	let snapshot = match locale {
		None => world
			.get_resource_ref::<TestCatalog>()
			.map(|resource| u64::from(resource.last_changed().get())),
		Some(locale) => world
			.get_resource::<LocaleSnapshots<TestCatalog>>()
			.and_then(|snapshots| {
				snapshots
					.ready
					.iter()
					.find(|snapshot| snapshot.locale == locale)
			})
			.map(|snapshot| snapshot.stamp),
	};
	Deferred {
		entity,
		message: message.identity(),
		locale,
		snapshot,
		text: Some("old".into()),
		changed: true,
		read: true,
		applied: false,
	}
}

#[test]
fn valid_queued_work_still_reaches_the_adapter() {
	let (mut world, entity, message) = scene(None);
	let item = work(&world, entity, &message, None);
	queue::<TestCatalog, Caption>(
		&mut world.commands(),
		vec![item],
		Some(TestCatalog("old".into())),
		None,
	);
	world.flush();
	assert_eq!(world.get::<Caption>(entity).unwrap().0, "old");
}

#[test]
fn changing_locale_with_the_same_message_rejects_queued_work() {
	let (mut world, entity, message) = scene(None);
	let item = work(&world, entity, &message, None);
	queue::<TestCatalog, Caption>(
		&mut world.commands(),
		vec![item],
		Some(TestCatalog("old".into())),
		None,
	);
	world
		.entity_mut(entity)
		.insert(Localized::<Caption, _>::from(message).with_locale("es"));
	world.flush();
	assert_eq!(world.get::<Caption>(entity).unwrap().0, "untouched");
}

#[test]
fn replacing_the_active_snapshot_rejects_queued_work() {
	let (mut world, entity, message) = scene(None);
	let item = work(&world, entity, &message, None);
	queue::<TestCatalog, Caption>(
		&mut world.commands(),
		vec![item],
		Some(TestCatalog("old".into())),
		None,
	);
	world.increment_change_tick();
	world.insert_resource(TestCatalog("new".into()));
	world.flush();
	assert_eq!(world.get::<Caption>(entity).unwrap().0, "untouched");
}

#[test]
fn replacing_a_fixed_snapshot_rejects_queued_work() {
	let (mut world, entity, message) = scene(Some("es"));
	let old: crate::catalog::SharedScope = Arc::new(TestCatalog("old".into()));
	LocaleSnapshots::<TestCatalog>::publish(&mut world, "es", Some(&old));
	let snapshots = world
		.resource::<LocaleSnapshots<TestCatalog>>()
		.ready
		.clone();
	let item = work(&world, entity, &message, Some("es"));
	queue::<TestCatalog, Caption>(&mut world.commands(), vec![item], None, Some(snapshots));
	let fresh: crate::catalog::SharedScope = Arc::new(TestCatalog("new".into()));
	LocaleSnapshots::<TestCatalog>::publish(&mut world, "es", Some(&fresh));
	world.flush();
	assert_eq!(world.get::<Caption>(entity).unwrap().0, "untouched");
}
