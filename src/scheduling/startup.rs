use super::{
	configuration::{StartupConfiguration, StartupStrategy},
	system::{self, DiscoveredSystem, Members, NodeState, TrackedCondition},
};
use crate::bevy::{
	ecs::{
		schedule::{
			ConditionWithAccess, FlattenedDependencies, InternedSystemSet, NodeId,
			ScheduleBuildError, ScheduleBuildPass, ScheduleGraph, SystemKey, SystemSetKey,
			SystemWithAccess,
			graph::{DiGraph, Direction},
		},
		system::{IntoSystem, System},
	},
	platform::hash::FixedHasher,
	prelude::*,
};
use indexmap::IndexSet;
use std::{
	collections::{HashMap, HashSet},
	sync::{Arc, Mutex, atomic::Ordering},
};

pub(super) type States = Arc<Mutex<HashMap<SystemKey, Arc<NodeState>>>>;

#[derive(Default)]
pub(super) struct Tracking {
	pub(super) states: States,
	sets: HashMap<SystemSetKey, (usize, Members)>,
	installed: bool,
	signature: Option<(usize, usize, usize)>,
}

fn descendants(graph: &ScheduleGraph, set: InternedSystemSet) -> HashSet<SystemKey> {
	let Some(key) = graph.system_sets.get_key(set) else {
		return HashSet::new();
	};
	descendants_of(graph, key)
}

fn descendants_of(graph: &ScheduleGraph, set: SystemSetKey) -> HashSet<SystemKey> {
	let mut pending = vec![NodeId::Set(set)];
	let mut visited = HashSet::new();
	let mut systems = HashSet::new();
	while let Some(node) = pending.pop() {
		if !visited.insert(node) {
			continue;
		}
		match node {
			NodeId::System(key) => {
				systems.insert(key);
			}
			NodeId::Set(_) => pending.extend(
				graph
					.hierarchy()
					.graph()
					.neighbors_directed(node, Direction::Outgoing),
			),
		}
	}
	systems
}

pub(super) fn prepare(
	schedule: &mut Schedule,
	tracking: &mut Tracking,
	startup: bool,
	policy: StartupConfiguration,
) {
	if startup && matches!(policy.strategy, StartupStrategy::Native) {
		return;
	}
	let deferred = startup;
	if !tracking.installed {
		schedule.add_build_pass(Dependencies {
			states: tracking.states.clone(),
			processed: HashSet::new(),
			startup: deferred,
			barrier: deferred.then_some(policy.barrier).flatten(),
		});
		tracking.installed = true;
	}
	let graph = schedule.graph_mut();
	let signature = (
		graph.systems.len(),
		graph.system_sets.len(),
		graph.hierarchy().graph().edge_count(),
	);
	if tracking.signature == Some(signature)
		&& graph.systems.is_initialized()
		&& graph.system_sets.is_initialized()
	{
		return;
	}
	tracking.signature = Some(signature);
	let existing: HashSet<_> = graph
		.hierarchy()
		.graph()
		.nodes()
		.filter_map(|node| match node {
			NodeId::System(key) => Some(key),
			NodeId::Set(_) => None,
		})
		.collect();
	tracking.states.lock().unwrap().retain(|key, state| {
		if existing.contains(key) {
			true
		} else {
			state.cancel();
			false
		}
	});
	let excluded = descendants(graph, crate::systems::ExplicitReadiness.intern());

	let keys: Vec<_> = graph
		.systems
		.iter()
		.map(|(key, _, _)| key)
		.filter(|key| !excluded.contains(key) && !tracking.states.lock().unwrap().contains_key(key))
		.collect();
	for key in &keys {
		let state = NodeState::new(deferred);
		let slot = graph.systems.get_mut(*key).unwrap();
		let original = std::mem::replace(
			slot,
			SystemWithAccess::new(Box::new(IntoSystem::into_system(|| {}))),
		);
		// Native initialization stays at the schedule's normal execution point.
		// The wrapper captures its returned metadata without a second native call.
		let wrapper = DiscoveredSystem::new(Box::new(original), state.clone());
		*slot = SystemWithAccess::new(Box::new(wrapper));
		let conditions = graph.systems.get_conditions_mut(*key).unwrap();
		for condition in conditions.iter_mut() {
			let original = std::mem::replace(
				&mut condition.condition,
				Box::new(IntoSystem::into_system(|| true)),
			);
			let wrapper =
				TrackedCondition::new(original, Arc::new(Mutex::new(vec![state.clone()])), false);
			condition.condition = Box::new(wrapper);
		}
		tracking.states.lock().unwrap().insert(*key, state);
	}
	{
		let sets: Vec<_> = graph.system_sets.iter().map(|(key, _, _)| key).collect();
		for set in sets {
			let members = descendants_of(graph, set);
			let states: Vec<_> = tracking
				.states
				.lock()
				.unwrap()
				.iter()
				.filter(|(key, _)| members.contains(key))
				.map(|(_, state)| state.clone())
				.collect();
			if states.is_empty() {
				continue;
			}
			let record = tracking
				.sets
				.entry(set)
				.or_insert_with(|| (0, Arc::new(Mutex::new(Vec::new()))));
			*record.1.lock().unwrap() = states;
			let seen = record.0;
			let conditions = graph.system_sets.get_conditions_mut(set).unwrap();
			for condition in conditions.iter_mut().skip(seen) {
				let original = std::mem::replace(
					&mut condition.condition,
					Box::new(IntoSystem::into_system(|| true)),
				);
				let wrapper = TrackedCondition::new(original, record.1.clone(), deferred);
				condition.condition = Box::new(wrapper);
			}
			record.0 = conditions.len();
		}
	}
}

struct Dependencies {
	states: States,
	processed: HashSet<SystemKey>,
	startup: bool,
	barrier: Option<InternedSystemSet>,
}

impl std::fmt::Debug for Dependencies {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str("LocalizationStartupDependencies")
	}
}

impl ScheduleBuildPass for Dependencies {
	type EdgeOptions = ();

	fn add_dependency(&mut self, _: NodeId, _: NodeId, _: Option<&Self::EdgeOptions>) {}

	fn collapse_set(
		&mut self,
		_: SystemSetKey,
		_: &IndexSet<SystemKey, FixedHasher>,
		_: &DiGraph<NodeId>,
	) -> impl Iterator<Item = (NodeId, NodeId)> {
		std::iter::empty()
	}

	fn build(
		&mut self,
		world: &mut World,
		graph: &mut ScheduleGraph,
		dependencies: FlattenedDependencies<'_>,
	) -> Result<(), ScheduleBuildError> {
		let states = self.states.lock().unwrap();
		for (key, state) in states.iter().filter(|(_, state)| state.startup) {
			let mut pending: Vec<_> = dependencies
				.graph()
				.neighbors_directed(*key, Direction::Incoming)
				.collect();
			let mut visited = HashSet::new();
			let mut predecessors = Vec::new();
			while let Some(key) = pending.pop() {
				if !visited.insert(key) {
					continue;
				}
				if let Some(previous) = states.get(&key) {
					predecessors.push(Arc::downgrade(previous));
				}
				pending.extend(
					dependencies
						.graph()
						.neighbors_directed(key, Direction::Incoming),
				);
			}
			*state.predecessors.lock().unwrap() = predecessors;
		}
		drop(states);
		let barrier = self
			.barrier
			.map(|set| descendants(graph, set))
			.unwrap_or_default();
		let deferred = self.startup;
		let keys: Vec<_> = self.states.lock().unwrap().keys().copied().collect();
		let barrier_states: Vec<_> = self
			.states
			.lock()
			.unwrap()
			.iter()
			.filter(|(key, _)| barrier.contains(key))
			.map(|(_, state)| state.clone())
			.collect();
		for state in &barrier_states {
			*state.retain_until.lock().unwrap() =
				barrier_states.iter().map(Arc::downgrade).collect();
		}
		let barrier_probes: Vec<_> = self
			.states
			.lock()
			.unwrap()
			.iter()
			.filter(|(key, state)| barrier.contains(key) && !state.done.load(Ordering::Acquire))
			.flat_map(|(_, state)| state.probes.lock().unwrap().clone())
			.collect();
		for key in keys {
			let fresh = self.processed.insert(key);
			if !fresh && !barrier.contains(&key) {
				continue;
			}
			let state = self.states.lock().unwrap()[&key].clone();

			let probes = if barrier.contains(&key) {
				barrier_probes.clone()
			} else {
				state.probes.lock().unwrap().clone()
			};
			if probes.is_empty() && !deferred {
				continue;
			}
			let mut gate = ConditionWithAccess::new(system::gate(world, state, probes));
			gate.access = gate.initialize(world);
			let conditions = graph.systems.get_conditions_mut(key).unwrap();
			let own_gate = (!fresh)
				.then(|| {
					conditions
						.iter()
						.position(|condition| condition.system_type() == gate.system_type())
				})
				.flatten();
			if let Some(index) = own_gate {
				conditions[index] = gate;
			} else {
				conditions.insert(0, gate);
			}
		}
		Ok(())
	}
}

pub(super) fn finished(tracking: &Tracking) -> bool {
	tracking
		.states
		.lock()
		.unwrap()
		.values()
		.all(|state| state.done.load(Ordering::Acquire))
}

pub(super) fn release_completed(tracking: &Tracking) {
	for state in tracking
		.states
		.lock()
		.unwrap()
		.values()
		.filter(|state| state.startup)
	{
		state.release_completed();
	}
}
