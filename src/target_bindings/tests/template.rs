use crate::Msg;
use crate::bevy::{
	ecs as bevy_ecs,
	ecs::template::{FromTemplate, SceneEntityReferences, Template, TemplateContext},
	prelude::{Component, Text, World},
};
use crate::tests::TestCatalog;
use crate::{LocalizationTarget, Localized};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

type TextTemplate = <Localized<Text, TestCatalog> as FromTemplate>::Template;

#[test]
fn canonical_template_clones_share_owned_arguments_and_defer_formatting() {
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let name = String::from("Ada");
	let template = TextTemplate::new(move |catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		format!("{name}:{}", catalog.0)
	});
	let cloned = template.clone_template();
	let mut world = World::new();
	let mut references = SceneEntityReferences::default();
	let mut entity = world.spawn_empty();
	let mut context = TemplateContext::new(&mut entity, &mut references);
	let first = template.build_template(&mut context).unwrap();
	let second = cloned.build_template(&mut context).unwrap();
	assert_eq!(calls.load(Ordering::Relaxed), 0);
	assert_eq!(first.message.render(&TestCatalog("ja".into())), "Ada:ja");
	assert_eq!(second.message.render(&TestCatalog("es".into())), "Ada:es");
	assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn canonical_template_accepts_both_message_and_component_values() {
	let message = Msg::<TestCatalog>::new(|catalog| catalog.0.clone());
	let from_message = TextTemplate::from(message.clone());
	let from_binding = TextTemplate::from(Localized::<Text, _>::from(message));
	let mut world = World::new();
	let mut references = SceneEntityReferences::default();
	let mut entity = world.spawn_empty();
	let mut context = TemplateContext::new(&mut entity, &mut references);

	for template in [from_message, from_binding] {
		let binding = template.build_template(&mut context).unwrap();
		assert_eq!(binding.message.render(&TestCatalog("ja".into())), "ja");
	}
}

#[test]
fn default_template_requires_an_explicit_message() {
	let template = TextTemplate::default();
	let mut world = World::new();
	let mut references = SceneEntityReferences::default();
	let mut entity = world.spawn_empty();
	let mut context = TemplateContext::new(&mut entity, &mut references);
	let error = template.build_template(&mut context).err().unwrap();
	assert!(
		error
			.to_string()
			.contains("Localized requires a formatting closure or Msg")
	);
}

#[derive(Component, Default)]
struct LabelTarget;

impl LocalizationTarget for LabelTarget {
	type Data = ();
	type Filter = ();

	fn apply(
		_: &mut crate::bevy::prelude::EntityWorldMut<'_>,
		_update: &mut crate::LocalizationUpdate<'_>,
	) {
	}
}

#[test]
fn custom_default_target_templates_need_no_clone_and_reject_missing_messages() {
	type CustomTemplate = <Localized<LabelTarget, TestCatalog> as FromTemplate>::Template;
	let calls = Arc::new(AtomicUsize::new(0));
	let observed = calls.clone();
	let message = Msg::<TestCatalog>::new(move |catalog| {
		observed.fetch_add(1, Ordering::Relaxed);
		catalog.0.clone()
	});
	let template = CustomTemplate::from(message.clone());
	let cloned = template.clone_template();
	let from_binding = CustomTemplate::from(Localized::<LabelTarget, _>::from(message));
	let mut world = World::new();
	let mut references = SceneEntityReferences::default();
	let mut entity = world.spawn_empty();
	let mut context = TemplateContext::new(&mut entity, &mut references);
	let bindings: Vec<_> = [template, cloned, from_binding]
		.into_iter()
		.map(|template| template.build_template(&mut context).unwrap())
		.collect();
	assert_eq!(calls.load(Ordering::Relaxed), 0);

	for binding in bindings {
		assert_eq!(binding.message.render(&TestCatalog("ja".into())), "ja");
	}

	assert_eq!(calls.load(Ordering::Relaxed), 3);
	assert!(
		CustomTemplate::default()
			.build_template(&mut context)
			.err()
			.unwrap()
			.to_string()
			.contains("Localized requires a formatting closure or Msg")
	);
}
