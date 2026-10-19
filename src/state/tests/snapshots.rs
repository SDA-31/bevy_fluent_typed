use super::LocaleSnapshots;
use crate::bevy::prelude::World;
use crate::catalog::SharedScope;
use crate::tests::TestCatalog;
use std::sync::Arc;

#[test]
fn publication_wrap_preserves_ready_updates_and_removal() {
	let mut world = World::new();
	let first: SharedScope = Arc::new(TestCatalog("first".into()));
	LocaleSnapshots::<TestCatalog>::publish(&mut world, "es", Some(&first));
	world.resource_mut::<LocaleSnapshots<TestCatalog>>().next = u64::MAX - 1;

	let second: SharedScope = Arc::new(TestCatalog("second".into()));
	LocaleSnapshots::<TestCatalog>::publish(&mut world, "es", Some(&second));
	let before = world.resource::<LocaleSnapshots<TestCatalog>>().ready[0].stamp;
	let latest: SharedScope = Arc::new(TestCatalog("latest".into()));
	LocaleSnapshots::<TestCatalog>::publish(&mut world, "es", Some(&latest));
	let table = world.resource::<LocaleSnapshots<TestCatalog>>();
	assert_ne!(table.ready[0].stamp, before);
	assert_eq!(table.ready[0].value.0, "latest");

	LocaleSnapshots::<TestCatalog>::publish(&mut world, "es", None);
	assert!(
		world
			.resource::<LocaleSnapshots<TestCatalog>>()
			.ready
			.is_empty()
	);
}
