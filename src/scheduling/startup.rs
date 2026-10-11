use super::{
	configuration::{StartupConfiguration, StartupStrategy},
	system::{self, DiscoveredSystem, Members, NodeState, Probes, TrackedCondition},
};
use crate::bevy::{
	ecs::{
		self as bevy_ecs,
		schedule::{
			ApplyDeferred, ConditionWithAccess, FlattenedDependencies, InternedSystemSet, NodeId,
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
	any::TypeId,
	collections::{HashMap, HashSet},
	sync::{
		Arc, Mutex, Weak,
		atomic::{AtomicUsize, Ordering},
	},
};

pub(super) type States = Arc<Mutex<HashMap<SystemKey, Arc<NodeState>>>>;
type Sets = Arc<Mutex<HashMap<SystemSetKey, SetConditions>>>;

#[derive(Default)]
pub(super) struct Tracking {
	pub(super) states: States,
	sets: Sets,
	identity: Option<ScheduleIdentity>,
	installed: bool,
	signature: Option<(usize, usize, usize)>,
}

#[derive(Default)]
pub(super) struct KnownSchedules(HashMap<ScheduleIdentity, SavedTracking>);

struct SavedTracking {
	states: Weak<Mutex<HashMap<SystemKey, Arc<NodeState>>>>,
	sets: Weak<Mutex<HashMap<SystemSetKey, SetConditions>>>,
	signature: Option<(usize, usize, usize)>,
}

#[derive(Default)]
struct SetConditions {
	seen: usize,
	members: Members,
	probes: Probes,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ScheduleIdentity(usize);

impl ScheduleIdentity {
	fn new() -> Self {
		static NEXT: AtomicUsize = AtomicUsize::new(0);
		Self(
			NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
				value.checked_add(1)
			})
			.expect("localization schedule identity counter exhausted"),
		)
	}
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
	known: &mut KnownSchedules,
	startup: bool,
	policy: StartupConfiguration,
) {
	if startup && matches!(policy.strategy, StartupStrategy::Native) {
		return;
	}
	let deferred = startup;
	if tracking
		.identity
		.is_some_and(|identity| !schedule.graph().system_sets.contains(identity))
	{
		// A new schedule may reuse both the label and native SystemKeys. The private
		// empty set identifies the graph without relying on allocation addresses.
		// Existing wrappers keep owning any old schedule retained by the application.
		*tracking = Tracking::default();
	}

	if !tracking.installed {
		known
			.0
			.retain(|_, saved| saved.states.strong_count() > 0 && saved.sets.strong_count() > 0);
		let saved = known.0.iter().find_map(|(identity, saved)| {
			if !schedule.graph().system_sets.contains(*identity) {
				return None;
			}

			Some((
				*identity,
				saved.states.upgrade()?,
				saved.sets.upgrade()?,
				saved.signature,
			))
		});
		if let Some((identity, states, sets, signature)) = saved {
			// The retained schedule's build pass owns this state. Restore it rather
			// than installing another pass or wrapping already discovered systems.
			*tracking = Tracking {
				identity: Some(identity),
				states,
				sets,
				signature,
				installed: true,
			};
		} else {
			let identity = ScheduleIdentity::new();
			schedule.configure_sets(identity);
			tracking.identity = Some(identity);
			schedule.add_build_pass(Dependencies {
				states: tracking.states.clone(),
				sets: tracking.sets.clone(),
				startup: deferred,
				barrier: deferred.then_some(policy.barrier).flatten(),
			});
			tracking.installed = true;
		}
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
	known.0.insert(
		tracking.identity.unwrap(),
		SavedTracking {
			states: Arc::downgrade(&tracking.states),
			sets: Arc::downgrade(&tracking.sets),
			signature: tracking.signature,
		},
	);
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
		let complete_in_gate = deferred
			&& graph.systems.get(*key).unwrap().system_type() == TypeId::of::<ApplyDeferred>();
		let state = NodeState::new(deferred, complete_in_gate);
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
			let wrapper = TrackedCondition::new(
				original,
				Arc::new(Mutex::new(vec![state.clone()])),
				false,
				None,
			);
			condition.condition = Box::new(wrapper);
		}
		tracking.states.lock().unwrap().insert(*key, state);
	}
	{
		let sets: Vec<_> = graph.system_sets.iter().map(|(key, _, _)| key).collect();
		tracking
			.sets
			.lock()
			.unwrap()
			.retain(|key, _| sets.contains(key));

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
			let mut records = tracking.sets.lock().unwrap();
			if states.is_empty() && !records.contains_key(&set) {
				continue;
			}

			let record = records.entry(set).or_default();
			*record.members.lock().unwrap() = states;
			let seen = record.seen;
			let conditions = graph.system_sets.get_conditions_mut(set).unwrap();
			for condition in conditions.iter_mut().skip(seen) {
				let original = std::mem::replace(
					&mut condition.condition,
					Box::new(IntoSystem::into_system(|| true)),
				);
				let wrapper = TrackedCondition::new(
					original,
					record.members.clone(),
					deferred,
					Some(record.probes.clone()),
				);
				condition.condition = Box::new(wrapper);
			}
			record.seen = conditions.len();
		}
	}
}

struct Dependencies {
	states: States,
	sets: Sets,
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
		let gate_owners: HashMap<_, _> = self
			.states
			.lock()
			.unwrap()
			.iter()
			.map(|(key, state)| {
				let mut held = state.gate_owners.lock().unwrap();
				let owners = held.upgrade().unwrap_or_default();
				*held = Arc::downgrade(&owners);
				(*key, owners)
			})
			.collect();

		// Set predicates initialize once. Their current descendants acquire the
		// inherited reads on each graph rebuild, including members added later.
		for record in self.sets.lock().unwrap().values() {
			let probes = record.probes.lock().unwrap().clone();
			for state in record.members.lock().unwrap().iter() {
				let owners = state.gate_owners.lock().unwrap().upgrade().unwrap();
				owners
					.lock()
					.unwrap()
					.extend(state.capture_probes(world, probes.clone()));
			}
		}

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
			let state = self.states.lock().unwrap()[&key].clone();

			let probes = if barrier.contains(&key) {
				barrier_probes.clone()
			} else {
				state.probes.lock().unwrap().clone()
			};
			if probes.is_empty() && !deferred {
				continue;
			}
			let owners = gate_owners[&key].clone();
			let complete_in_gate = state.complete_in_gate;
			let mut gate = ConditionWithAccess::new(system::gate(world, state, probes, owners));
			gate.access = gate.initialize(world);
			let conditions = graph.systems.get_conditions_mut(key).unwrap();
			let own_gate = conditions
				.iter()
				.position(|condition| condition.system_type() == gate.system_type());
			if let Some(index) = own_gate {
				conditions[index] = gate;
			} else if complete_in_gate {
				// A synchronization point has no body callback. Its native conditions
				// must evaluate before our final completion predicate changes done.
				conditions.push(gate);
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
