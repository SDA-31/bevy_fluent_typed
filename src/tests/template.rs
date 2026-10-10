use super::TestCatalog;
use crate::bevy::{
	ecs::template::{FromTemplate, SceneEntityReferences, Template, TemplateContext},
	prelude::World,
};
use crate::{LocalizedText, Message};
use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

type TextTemplate = <LocalizedText<TestCatalog> as FromTemplate>::Template;

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
	assert_eq!(first.0.render(&TestCatalog("ja".into())), "Ada:ja");
	assert_eq!(second.0.render(&TestCatalog("es".into())), "Ada:es");
	assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn canonical_template_accepts_both_message_and_component_values() {
	let message = Message::<TestCatalog>::new(|catalog| catalog.0.clone());
	let from_message = TextTemplate::from(message.clone());
	let from_binding = TextTemplate::from(LocalizedText::from(message));
	let mut world = World::new();
	let mut references = SceneEntityReferences::default();
	let mut entity = world.spawn_empty();
	let mut context = TemplateContext::new(&mut entity, &mut references);

	for template in [from_message, from_binding] {
		let binding = template.build_template(&mut context).unwrap();
		assert_eq!(binding.0.render(&TestCatalog("ja".into())), "ja");
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
			.contains("LocalizedText requires a formatting closure or Message")
	);
}
