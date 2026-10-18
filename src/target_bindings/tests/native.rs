use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{
		App, Component, DetectChanges, DetectChangesMut, MinimalPlugins, Resource, Text, Text2d,
		TextSpan,
	},
};
use crate::bindings::Binding;
use crate::{
	FluentCatalog, FluentScope, LocalizationTarget, Localized, Module, ModuleStore, Msg,
	ScopeRegistration,
};
use std::{
	ops::DerefMut,
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	},
};

pub(super) trait NativeText:
	LocalizationTarget
	+ Component<Mutability = crate::bevy::ecs::component::Mutable>
	+ Default
	+ DerefMut<Target = String>
{
}

impl NativeText for Text {}

impl NativeText for Text2d {}

impl NativeText for TextSpan {}

#[derive(Resource, Clone)]
pub(super) struct Catalog(pub(super) String);

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

fn exercise<T: NativeText>(count: usize, supplied: bool) {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("first".into()));
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let message = Msg::new(move |catalog: &Catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	});
	let owners: Vec<_> = (0..count)
		.map(|_| {
			let binding = Localized::<T, Catalog>::from(message.clone());
			let owner = if supplied {
				world.spawn((T::default(), binding))
			} else {
				world.spawn((binding, T::default()))
			};

			owner.id()
		})
		.collect();
	world.flush();
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count);
	let ticks: Vec<_> = owners
		.iter()
		.map(|&e| world.entity(e).get_ref::<T>().unwrap().last_changed())
		.collect();

	for _ in 0..3 {
		<Localized<T, Catalog> as Binding>::refresh(world);
	}

	assert_eq!(calls.load(Ordering::Relaxed), count);

	for (&owner, tick) in owners.iter().zip(ticks) {
		assert_eq!(world.get::<T>(owner).unwrap().deref(), "first");
		assert_eq!(
			world.entity(owner).get_ref::<T>().unwrap().last_changed(),
			tick
		);
	}

	world.get_mut::<T>(owners[0]).unwrap().deref_mut().clear();
	world
		.get_mut::<T>(owners[count - 1])
		.unwrap()
		.deref_mut()
		.clear();
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count);
	assert_eq!(world.get::<T>(owners[0]).unwrap().deref(), "first");
	assert_eq!(world.get::<T>(owners[count - 1]).unwrap().deref(), "first");
	world.insert_resource(Catalog("second".into()));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count * 2);

	for &owner in &owners {
		assert_eq!(world.get::<T>(owner).unwrap().deref(), "second");
	}

	world.remove_resource::<Catalog>();
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count * 2);
	assert!(world.get::<T>(owners[0]).unwrap().is_empty());
	world.insert_resource(Catalog("third".into()));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count * 3);
	world
		.entity_mut(owners[0])
		.insert(Localized::<T, Catalog>::from(message));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count * 3);
	let observed = calls.clone();
	world
		.entity_mut(owners[0])
		.insert(Localized::<T, Catalog>::new(move |catalog| {
			observed.fetch_add(1, Ordering::Relaxed);
			format!("{}!", catalog.0)
		}));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count * 3 + 1);
	assert_eq!(world.get::<T>(owners[0]).unwrap().deref(), "third!");

	#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
	{
		use crate::bevy::ecs::entity_disabling::Disabled;

		world.entity_mut(owners[0]).insert(Disabled);
		<Localized<T, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), count * 3 + 1);
		world.entity_mut(owners[0]).remove::<Disabled>();
		<Localized<T, Catalog> as Binding>::refresh(world);
		assert_eq!(calls.load(Ordering::Relaxed), count * 3 + 2);
	}

	let before = calls.load(Ordering::Relaxed);
	let tick = world
		.entity(owners[0])
		.get_ref::<T>()
		.unwrap()
		.last_changed();
	let observed = calls.clone();
	let replacement = Localized::<T, Catalog>::new(move |catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		format!("{}!", catalog.0)
	});
	*world
		.get_mut::<Localized<T, Catalog>>(owners[0])
		.unwrap()
		.bypass_change_detection() = replacement;
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), before + 1);
	assert_eq!(
		world
			.entity(owners[0])
			.get_ref::<T>()
			.unwrap()
			.last_changed(),
		tick
	);
	<Localized<T, Catalog> as Binding>::unregister(world);
}

#[test]
fn native_text_paths_preserve_cache_repairs_and_change_ticks() {
	for count in [1, 8192] {
		for supplied in [false, true] {
			exercise::<Text>(count, supplied);
			exercise::<Text2d>(count, supplied);
			exercise::<TextSpan>(count, supplied);
		}
	}

	exercise::<Text>(32_768, false);
}

fn exercise_native_target_tracking<T: NativeText>() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("original".into()));
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let owner = world
		.spawn((
			T::default(),
			Localized::<T, Catalog>::new(move |catalog| {
				observed.fetch_add(1, Ordering::Relaxed);
				catalog.0.clone()
			}),
		))
		.id();
	world.flush();
	<Localized<T, Catalog> as Binding>::refresh(world);
	let before = world.entity(owner).get_ref::<T>().unwrap().last_changed();
	world.increment_change_tick();

	{
		let mut target = world.get_mut::<T>(owner).unwrap();
		target
			.bypass_change_detection()
			.deref_mut()
			.replace_range(.., "replaced");
	}

	assert_eq!(
		world.entity(owner).get_ref::<T>().unwrap().last_changed(),
		before
	);
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<T>(owner).unwrap().deref(), "replaced");
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	world
		.get_mut::<T>(owner)
		.unwrap()
		.deref_mut()
		.replace_range(.., "ordinary");
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<T>(owner).unwrap().deref(), "original");
	assert_eq!(calls.load(Ordering::Relaxed), 1);
	world
		.get_mut::<T>(owner)
		.unwrap()
		.bypass_change_detection()
		.deref_mut()
		.replace_range(.., "replaced");
	world.insert_resource(Catalog("updated".into()));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(world.get::<T>(owner).unwrap().deref(), "updated");
	assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn native_targets_follow_bevy_change_detection() {
	exercise_native_target_tracking::<Text>();
	exercise_native_target_tracking::<Text2d>();
	exercise_native_target_tracking::<TextSpan>();
}

fn exercise_mixed_growth<T: NativeText>() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("short".into()));
	let calls = Arc::new(AtomicUsize::new(0));
	let count = 4 << 10;
	let owners: Vec<_> = (0..count)
		.map(|index| {
			let observed = calls.clone();
			world
				.spawn((
					T::default(),
					Localized::<T, Catalog>::new(move |catalog| {
						observed.fetch_add(1, Ordering::Relaxed);
						format!("{}-{index}", catalog.0)
					}),
				))
				.id()
		})
		.collect();
	world.flush();
	<Localized<T, Catalog> as Binding>::refresh(world);
	let value = "a longer localized caption".repeat(4);

	for (index, &owner) in owners.iter().enumerate() {
		if index % 3 == 2 {
			world.entity_mut(owner).insert(T::default());
			continue;
		}

		let mut target = world.get_mut::<T>(owner).unwrap();
		let text = target.deref_mut().deref_mut();

		if index % 3 == 0 {
			*text = "x".to_owned();
			text.shrink_to_fit();
		} else {
			text.reserve(value.len());
		}
	}

	world.insert_resource(Catalog(value.clone()));
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count * 2);

	for (index, &owner) in owners.iter().enumerate() {
		assert_eq!(
			world.get::<T>(owner).unwrap().deref(),
			&format!("{value}-{index}")
		);
	}

	let ticks: Vec<_> = owners
		.iter()
		.map(|&owner| world.entity(owner).get_ref::<T>().unwrap().last_changed())
		.collect();
	<Localized<T, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), count * 2);

	for (&owner, tick) in owners.iter().zip(ticks) {
		assert_eq!(
			world.entity(owner).get_ref::<T>().unwrap().last_changed(),
			tick
		);
	}
}

#[test]
fn mixed_native_growth_preserves_outputs_and_single_formatting() {
	exercise_mixed_growth::<Text>();
	exercise_mixed_growth::<Text2d>();
	exercise_mixed_growth::<TextSpan>();
}

#[test]
fn removed_binding_retires_its_cache_while_another_owner_survives() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("ready".into()));
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let message = Msg::new(move |catalog: &Catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	});
	let owners: Vec<_> = (0..2)
		.map(|_| {
			world
				.spawn((
					Text::default(),
					Localized::<Text, Catalog>::from(message.clone()),
				))
				.id()
		})
		.collect();
	world.flush();
	<Localized<Text, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), 2);
	world
		.entity_mut(owners[0])
		.remove::<Localized<Text, Catalog>>();
	world.flush();
	world
		.entity_mut(owners[0])
		.insert(Localized::<Text, Catalog>::from(message));
	world.flush();
	<Localized<Text, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), 3);
	assert_eq!(world.get::<Text>(owners[0]).unwrap().0, "ready");
	assert_eq!(world.get::<Text>(owners[1]).unwrap().0, "ready");
}

#[test]
fn bulk_message_replacement_is_detected_without_component_change_ticks() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	let world = app.world_mut();
	world.insert_resource(Catalog("stable".into()));
	let initial = Msg::new(|catalog: &Catalog| catalog.0.clone());
	let owners: Vec<_> = world
		.spawn_batch((0..32_768).map(|_| {
			(
				Text::default(),
				Localized::<Text, Catalog>::from(initial.clone()),
			)
		}))
		.collect();
	world.flush();
	<Localized<Text, Catalog> as Binding>::refresh(world);
	let tick = world
		.entity(owners[0])
		.get_ref::<Localized<Text, Catalog>>()
		.unwrap()
		.last_changed();
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let message = Msg::new(move |catalog: &Catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		format!("new {}", catalog.0)
	});

	for mut binding in world
		.query::<&mut Localized<Text, Catalog>>()
		.iter_mut(world)
	{
		*binding.bypass_change_detection() = Localized::from(message.clone());
	}

	<Localized<Text, Catalog> as Binding>::refresh(world);
	assert_eq!(calls.load(Ordering::Relaxed), owners.len());
	assert_eq!(
		world
			.entity(owners[0])
			.get_ref::<Localized<Text, Catalog>>()
			.unwrap()
			.last_changed(),
		tick
	);

	for owner in owners {
		assert_eq!(world.get::<Text>(owner).unwrap().0, "new stable");
	}
}
