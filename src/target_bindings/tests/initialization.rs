use super::tests::Catalog;
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{App, Component, EntityWorldMut, MinimalPlugins},
};
use crate::bindings::Binding;
use crate::{LocalizationTarget, LocalizationUpdate, Localized, Msg};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

#[derive(Component)]
struct Caption {
	text: String,
	size: u32,
}

impl Default for Caption {
	fn default() -> Self {
		Self {
			text: String::new(),
			size: 24,
		}
	}
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

#[test]
fn custom_default_targets_initialize_without_an_override_or_a_ready_catalog() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = Arc::clone(&calls);
	let message = Msg::<Catalog>::new(move |catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	});

	let binding = Localized::<Caption, _>::from(message.clone());
	let cloned = binding.clone();
	let observed = Arc::clone(&calls);
	let constructed = Localized::<Caption, Catalog>::new(move |catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	});

	let owners = [binding, cloned, constructed].map(|binding| world.spawn(binding).id());
	world.flush();

	for owner in owners {
		let caption = world.get::<Caption>(owner).unwrap();
		assert!(caption.text.is_empty());
		assert_eq!(caption.size, 24);
	}

	assert_eq!(calls.load(Ordering::Relaxed), 0);
	world.insert_resource(Catalog("ready".into()));
	<Localized<Caption, Catalog> as Binding>::refresh(world);

	for owner in owners {
		assert_eq!(world.get::<Caption>(owner).unwrap().text, "ready");
	}

	assert_eq!(calls.load(Ordering::Relaxed), owners.len());
}

#[test]
fn default_initialization_preserves_supplied_component_settings() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	let owner = world
		.spawn((
			Localized::<Caption, Catalog>::new(|catalog| catalog.0.clone()),
			Caption {
				text: "supplied".into(),
				size: 48,
			},
		))
		.id();
	world.flush();
	let caption = world.get::<Caption>(owner).unwrap();
	assert_eq!(caption.text, "supplied");
	assert_eq!(caption.size, 48);
	world.insert_resource(Catalog("translated".into()));
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	let caption = world.get::<Caption>(owner).unwrap();
	assert_eq!(caption.text, "translated");
	assert_eq!(caption.size, 48);
}

#[test]
fn removed_custom_targets_wait_until_reinserted_or_a_new_binding_lifetime() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("first".into()));
	let binding = Localized::<Caption, Catalog>::new(|catalog| catalog.0.clone());
	let owner = world.spawn(binding.clone()).id();
	world.flush();
	assert!(world.get::<Caption>(owner).is_some());
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	world.entity_mut(owner).remove::<Caption>();
	world.insert_resource(Catalog("second".into()));
	world.entity_mut(owner).insert(binding.clone());
	world.flush();
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	assert!(world.get::<Caption>(owner).is_none());
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
	world.entity_mut(owner).insert(binding);
	world.flush();
	let caption = world.get::<Caption>(owner).unwrap();
	assert!(caption.text.is_empty());
	assert_eq!(caption.size, 24);
	<Localized<Caption, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<Caption>(owner).unwrap().text, "second");
}
