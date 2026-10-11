#![cfg_attr(
	feature = "bevy-0-20",
	allow(
		deprecated,
		reason = "Readiness uses Bevy's retained filtered resource view."
	)
)]
use super::access;
use super::readiness::Probe;
use crate::bevy::{
	ecs::{
		change_detection::{CheckChangeTicks, Tick},
		schedule::{BoxedCondition, InternedSystemSet},
		system::{
			FilteredResourcesParamBuilder, ReadOnlySystem, RunSystemError, ScheduleSystem, System,
			SystemParamBuilder, SystemStateFlags,
		},
		world::{
			DeferredWorld, FilteredResources, FilteredResourcesBuilder, WorldId,
			unsafe_world_cell::UnsafeWorldCell,
		},
	},
	prelude::World,
	utils::prelude::DebugName,
};
use crate::compatibility::scheduling::SystemAccess;
use crate::demand::Consumer;
use std::{
	any::TypeId,
	sync::{
		Arc, Mutex, Weak,
		atomic::{AtomicBool, Ordering},
	},
};

pub(super) struct NodeState {
	pub(super) startup: bool,
	pub(super) complete_in_gate: bool,
	pub(super) done: AtomicBool,
	pub(super) predecessors: Mutex<Vec<Weak<NodeState>>>,
	pub(super) retain_until: Mutex<Vec<Weak<NodeState>>>,
	pub(super) probes: Mutex<Vec<Arc<Probe>>>,
	pub(super) gate_owners: Mutex<Weak<Mutex<Vec<Arc<Consumer>>>>>,
	owners: Mutex<Vec<Weak<Consumer>>>,
}

impl NodeState {
	pub(super) fn new(startup: bool, complete_in_gate: bool) -> Arc<Self> {
		Arc::new(Self {
			startup,
			complete_in_gate,
			done: AtomicBool::new(false),
			predecessors: Mutex::new(Vec::new()),
			retain_until: Mutex::new(Vec::new()),
			probes: Mutex::new(Vec::new()),
			gate_owners: Mutex::new(Weak::new()),
			owners: Mutex::new(Vec::new()),
		})
	}

	pub(super) fn complete(&self) {
		self.done.store(true, Ordering::Release);
	}

	pub(super) fn cancel(&self) {
		self.complete();
		self.retain_until.lock().unwrap().clear();
		self.release_completed();
	}

	pub(super) fn release_completed(&self) {
		if self.done.load(Ordering::Acquire)
			&& self.retain_until.lock().unwrap().iter().all(|state| {
				state
					.upgrade()
					.is_none_or(|state| state.done.load(Ordering::Acquire))
			}) {
			for owner in self.owners.lock().unwrap().iter().filter_map(Weak::upgrade) {
				owner.release();
			}
		}
	}

	pub(super) fn predecessors_done(&self) -> bool {
		self.predecessors.lock().unwrap().iter().all(|state| {
			state
				.upgrade()
				.is_none_or(|state| state.done.load(Ordering::Acquire))
		})
	}

	fn capture(&self, world: &mut World, native: &SystemAccess) -> Vec<Arc<Consumer>> {
		self.capture_probes(world, access::probes(world, native))
	}

	pub(super) fn capture_probes(
		&self,
		world: &mut World,
		probes: Vec<Arc<Probe>>,
	) -> Vec<Arc<Consumer>> {
		if self.done.load(Ordering::Acquire) {
			return Vec::new();
		}

		let mut known = self.probes.lock().unwrap();
		let mut owners = Vec::new();
		for probe in probes {
			if known.iter().any(|existing| existing.scope == probe.scope) {
				continue;
			}

			if let Some(acquire) = probe.consumer {
				owners.push(acquire(world));
			}
			known.push(probe);
		}

		self.owners
			.lock()
			.unwrap()
			.extend(owners.iter().map(Arc::downgrade));
		owners
	}
}

macro_rules! delegate {
	() => {
		fn name(&self) -> DebugName {
			self.inner.name()
		}

		fn system_type(&self) -> TypeId {
			self.inner.system_type()
		}

		fn flags(&self) -> SystemStateFlags {
			self.inner.flags()
		}

		fn apply_deferred(&mut self, world: &mut World) {
			self.inner.apply_deferred(world);
		}

		fn queue_deferred(&mut self, world: DeferredWorld) {
			self.inner.queue_deferred(world);
		}

		fn check_change_tick(&mut self, check: CheckChangeTicks) {
			self.inner.check_change_tick(check);
		}

		fn default_system_sets(&self) -> Vec<InternedSystemSet> {
			self.inner.default_system_sets()
		}

		fn get_last_run(&self) -> Tick {
			self.inner.get_last_run()
		}

		fn set_last_run(&mut self, tick: Tick) {
			self.inner.set_last_run(tick);
		}

		#[cfg(feature = "hotpatching")]
		fn refresh_hotpatch(&mut self) {
			self.inner.refresh_hotpatch();
		}
	};
}

pub(super) struct DiscoveredSystem {
	inner: ScheduleSystem,
	state: Arc<NodeState>,
	access: Option<(WorldId, SystemAccess)>,
	owners: Vec<Arc<Consumer>>,
}

impl DiscoveredSystem {
	pub(super) fn new(inner: ScheduleSystem, state: Arc<NodeState>) -> Self {
		Self {
			inner,
			state,
			access: None,
			owners: Vec::new(),
		}
	}
}

impl System for DiscoveredSystem {
	type In = ();
	type Out = ();
	delegate!();

	unsafe fn run_unsafe(
		&mut self,
		input: (),
		world: UnsafeWorldCell,
	) -> Result<(), RunSystemError> {
		assert_eq!(
			self.access.as_ref().map(|(world, _)| *world),
			Some(world.id()),
			"localization systems cannot move between Worlds after initialization"
		);
		// SAFETY: The wrapper preserves the inner system's complete scheduler access.
		let result = unsafe { self.inner.run_unsafe(input, world) };
		if self.state.startup {
			self.state.complete();
		}
		result
	}

	fn initialize(&mut self, world: &mut World) -> SystemAccess {
		if let Some((expected, access)) = &self.access {
			assert_eq!(
				*expected,
				world.id(),
				"localization systems cannot move between Worlds after initialization"
			);
			return access.clone();
		}
		let access = self.inner.initialize(world);
		#[cfg(feature = "bevy-0-20")]
		let access = {
			let mut access = access;
			// The wrapper checks WorldId even when the native system has no data access.
			access.ensure_metadata_access();
			access
		};
		self.owners = self.state.capture(world, &access);
		self.access = Some((world.id(), access.clone()));
		access
	}
}

pub(super) type Members = Arc<Mutex<Vec<Arc<NodeState>>>>;
pub(super) type Probes = Arc<Mutex<Vec<Arc<Probe>>>>;
pub(super) type GateOwners = Arc<Mutex<Vec<Arc<Consumer>>>>;

pub(super) struct TrackedCondition {
	inner: BoxedCondition,
	states: Members,
	shared_probes: Option<Probes>,
	cache_result: bool,
	result: Option<bool>,
	access: Option<(WorldId, SystemAccess)>,
	owners: Vec<Arc<Consumer>>,
	catalog_gate: Option<BoxedCondition>,
}

impl TrackedCondition {
	pub(super) fn new(
		inner: BoxedCondition,
		states: Members,
		cache_result: bool,
		shared_probes: Option<Probes>,
	) -> Self {
		Self {
			inner,
			states,
			shared_probes,
			cache_result,
			result: None,
			access: None,
			owners: Vec::new(),
			catalog_gate: None,
		}
	}
}

impl System for TrackedCondition {
	type In = ();
	type Out = bool;
	delegate!();

	unsafe fn run_unsafe(
		&mut self,
		input: (),
		world: UnsafeWorldCell,
	) -> Result<bool, RunSystemError> {
		assert_eq!(
			self.access.as_ref().map(|(world, _)| *world),
			Some(world.id()),
			"localization conditions cannot move between Worlds after initialization"
		);
		let states = self.states.lock().unwrap();
		if states
			.iter()
			.all(|state| state.startup && state.done.load(Ordering::Acquire))
		{
			return Ok(false);
		}
		if let Some(result) = self.result {
			if !result {
				for state in states.iter().filter(|state| state.startup) {
					state.complete();
				}
			}
			return Ok(result);
		}
		// Bevy evaluates all predicates even when a preceding readiness gate is false.
		// A shared set can start as soon as one member's external predecessors finish;
		// requiring every member would deadlock A -> outside C -> B in the same set.
		let eligible = states.iter().any(|state| {
			!state.startup
				|| (!state.done.load(Ordering::Acquire)
					&& state.predecessors.lock().unwrap().iter().all(|previous| {
						previous.upgrade().is_none_or(|previous| {
							previous.done.load(Ordering::Acquire)
								|| (self.shared_probes.is_some()
									&& states.iter().any(|member| Arc::ptr_eq(member, &previous)))
						})
					}))
		});
		if !eligible {
			return Ok(false);
		}

		if let Some(gate) = &mut self.catalog_gate {
			// SAFETY: initialize extends the condition access with this read-only gate.
			if !unsafe { gate.run_unsafe((), world) }? {
				return Ok(false);
			}
		}
		// SAFETY: Native condition access and read-only behavior are preserved.
		let result = unsafe { self.inner.run_unsafe(input, world) };
		if result.as_ref().is_ok_and(|value| !*value) || result.is_err() {
			for state in states.iter() {
				if state.startup {
					state.complete();
				}
			}
		}
		if self.cache_result || states.iter().any(|state| state.startup) {
			self.result = result.as_ref().ok().copied();
		}
		result
	}

	fn initialize(&mut self, world: &mut World) -> SystemAccess {
		if let Some((expected, access)) = &self.access {
			assert_eq!(
				*expected,
				world.id(),
				"localization systems cannot move between Worlds after initialization"
			);
			return access.clone();
		}
		let mut access = self.inner.initialize(world);
		#[cfg(feature = "bevy-0-20")]
		access.ensure_metadata_access();
		let probes = access::probes(world, &access);
		if !probes.is_empty() {
			let mut gate = readiness_gate(world, probes.clone());
			access.extend(gate.initialize(world));
			self.catalog_gate = Some(gate);
		}
		if let Some(shared) = &self.shared_probes {
			shared.lock().unwrap().extend(probes);
		} else {
			for state in self.states.lock().unwrap().iter() {
				self.owners
					.extend(state.capture_probes(world, probes.clone()));
			}
		}
		self.access = Some((world.id(), access.clone()));
		access
	}
}

// SAFETY: The original condition is read-only; the wrapper mutates only its owned state.
unsafe impl ReadOnlySystem for TrackedCondition {}

pub(super) fn gate(
	world: &mut World,
	state: Arc<NodeState>,
	probes: Vec<Arc<Probe>>,
	owners: GateOwners,
) -> BoxedCondition {
	let reads = probes.clone();
	let builder =
		FilteredResourcesParamBuilder::new(move |builder: &mut FilteredResourcesBuilder| {
			for probe in reads {
				(probe.access)(builder);
			}
		});
	Box::new(
		(builder,)
			.build_state(world)
			.build_system(move |resources: FilteredResources| {
				// The gate, rather than global tracking, owns inherited set-condition demand.
				let _held_owners = &owners;
				let ready = !state.done.load(Ordering::Acquire)
					&& (!state.startup || state.predecessors_done())
					&& probes.iter().all(|probe| (probe.ready)(&resources));

				// Bevy executes ApplyDeferred itself, without calling the system body.
				// Its final predicate records completion after native conditions ran;
				// the executor still applies commands before releasing dependents.
				if ready && state.complete_in_gate {
					state.complete();
				}

				ready
			}),
	)
}

fn readiness_gate(world: &mut World, probes: Vec<Arc<Probe>>) -> BoxedCondition {
	let reads = probes.clone();
	let builder =
		FilteredResourcesParamBuilder::new(move |builder: &mut FilteredResourcesBuilder| {
			for probe in reads {
				(probe.access)(builder);
			}
		});
	Box::new(
		(builder,)
			.build_state(world)
			.build_system(move |resources: FilteredResources| {
				probes.iter().all(|probe| (probe.ready)(&resources))
			}),
	)
}
