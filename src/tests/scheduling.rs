//! Plugin-level regression: Update changes reach both text pipelines this frame.
use super::TestCatalog;
use crate::bevy::ecs::schedule::{NodeId, ScheduleGraph};
use crate::bevy::ecs::system::System;
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{Localization, LocalizationPlugin, LocalizedText, bindings};
use std::collections::HashSet;

#[derive(Resource, Default)]
struct Seen(Vec<(String, String)>);

fn switch(mut switched: Local<bool>, mut localization: ResMut<Localization<TestCatalog>>) {
	if *switched {
		return;
	}

	localization.set_locale("es");
	*switched = true;
}

fn observe(ui: Query<&Text>, world: Query<&Text2d>, mut seen: ResMut<Seen>) {
	seen.0.push((
		ui.single().unwrap().0.clone(),
		world.single().unwrap().0.clone(),
	));
}

#[test]
// Shared with 0.17/0.18, which lack 0.19's replacement System::system_type API.
#[allow(deprecated)]
fn update_changes_reach_ui_and_world_before_engine_text_detection() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.add_plugins(LocalizationPlugin::<TestCatalog>::new(
			"missing-test.definition",
		))
		.init_resource::<Seen>()
		.add_systems(Update, switch);

	#[cfg(feature = "bevy-0-19")]
	app.add_systems(
		PostUpdate,
		(
			crate::bevy::text::detect_text_needs_rerender,
			observe.after(crate::bevy::text::detect_text_needs_rerender),
		),
	);

	#[cfg(not(feature = "bevy-0-19"))]
	app.add_systems(
		PostUpdate,
		(
			crate::bevy::text::detect_text_needs_rerender::<Text>,
			crate::bevy::text::detect_text_needs_rerender::<Text2d>,
			observe
				.after(crate::bevy::text::detect_text_needs_rerender::<Text>)
				.after(crate::bevy::text::detect_text_needs_rerender::<Text2d>),
		),
	);
	let ui = app
		.world_mut()
		.spawn((
			Text::default(),
			LocalizedText::<TestCatalog>::new(|catalog| catalog.0.clone()),
			Name::new("preserved"),
		))
		.id();
	app.world_mut().spawn((
		Text2d::default(),
		LocalizedText::<TestCatalog>::new(|catalog| catalog.0.clone()),
	));
	app.finish();
	app.cleanup();
	app.update();
	let schedules = app.world().resource::<Schedules>();
	let schedule = schedules.get(PostUpdate).unwrap();
	let nodes: Vec<_> = schedule
		.systems()
		.unwrap()
		.map(|(key, _)| NodeId::System(key))
		.collect();
	let systems: Vec<_> = schedules
		.get(PostUpdate)
		.unwrap()
		.systems()
		.unwrap()
		.map(|(_, system)| system.type_id())
		.collect();
	let ui_refresh = systems
		.iter()
		.position(|&id| {
			id == IntoSystem::into_system(bindings::refresh_ui::<TestCatalog>).type_id()
		})
		.unwrap();
	let world_refresh = systems
		.iter()
		.position(|&id| {
			id == IntoSystem::into_system(bindings::refresh_world::<TestCatalog>).type_id()
		})
		.unwrap();
	#[cfg(feature = "bevy-0-19")]
	let detector_types =
		[IntoSystem::into_system(crate::bevy::text::detect_text_needs_rerender).type_id()];
	#[cfg(not(feature = "bevy-0-19"))]
	let detector_types = [
		IntoSystem::into_system(crate::bevy::text::detect_text_needs_rerender::<Text>).type_id(),
		IntoSystem::into_system(crate::bevy::text::detect_text_needs_rerender::<Text2d>).type_id(),
	];
	let detectors: Vec<_> = systems
		.iter()
		.enumerate()
		.filter(|(_, id)| detector_types.contains(id))
		.map(|(index, _)| index)
		.collect();
	assert_eq!(detectors.len(), detector_types.len());
	let ordering = system_dependencies(schedule.graph());

	for index in detectors {
		assert!(reachable(&ordering, nodes[ui_refresh]).contains(&nodes[index]));
		assert!(reachable(&ordering, nodes[world_refresh]).contains(&nodes[index]));
	}

	assert_eq!(
		app.world().resource::<Seen>().0,
		[("es".into(), "es".into())]
	);
	assert_eq!(app.world().get::<Name>(ui).unwrap().as_str(), "preserved");
	app.update(); // Settle change detection before testing a binding-only change.

	app.world_mut()
		.entity_mut(ui)
		.insert(LocalizedText::<TestCatalog>::new(|catalog| {
			format!("{}!", catalog.0)
		}));
	app.update();
	assert_eq!(
		app.world().resource::<Seen>().0[2],
		("es!".into(), "es".into())
	);
}

/// Expand set-to-set constraints into actual system dependencies. Membership
/// itself is not an ordering edge: peers inside one set may execute either way.
fn system_dependencies(graph: &ScheduleGraph) -> Vec<(NodeId, NodeId)> {
	let hierarchy: Vec<_> = graph.hierarchy().graph().all_edges().collect();
	let mut edges = Vec::new();

	for (before, after) in graph.dependency().graph().all_edges() {
		let sources = reachable(&hierarchy, before);
		let targets = reachable(&hierarchy, after);

		for source in sources
			.into_iter()
			.filter(|node| matches!(node, NodeId::System(_)))
		{
			for &target in targets
				.iter()
				.filter(|node| matches!(node, NodeId::System(_)))
			{
				edges.push((source, target));
			}
		}
	}

	edges
}

fn reachable(edges: &[(NodeId, NodeId)], start: NodeId) -> HashSet<NodeId> {
	let mut visited = HashSet::new();
	let mut pending = vec![start];

	while let Some(node) = pending.pop() {
		if !visited.insert(node) {
			continue;
		}

		pending.extend(
			edges
				.iter()
				.filter(|(from, _)| *from == node)
				.map(|(_, to)| *to),
		);
	}

	visited
}
