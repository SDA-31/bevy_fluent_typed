//! Narrow, dynamically selected read access for inferred catalog parameters.
use crate::bevy::{
	ecs::{
		system::{
			FilteredResourcesParamBuilder, ReadOnlySystemParam, SystemMeta, SystemParam,
			SystemParamBuilder,
		},
		world::{FilteredResources, FilteredResourcesBuilder, unsafe_world_cell::UnsafeWorldCell},
	},
	prelude::World,
};
use crate::systems::{CatalogReadiness, ParameterTypes};
use std::{marker::PhantomData, sync::Arc};

#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19"))]
use crate::bevy::ecs::change_detection::Tick;
#[cfg(not(any(feature = "bevy-0-18", feature = "bevy-0-19")))]
use crate::bevy::ecs::component::Tick;

pub(crate) struct Probe {
	pub(crate) access: Box<dyn Fn(&mut FilteredResourcesBuilder) + Send + Sync>,
	pub(crate) ready: Box<dyn Fn(&FilteredResources) -> bool + Send + Sync>,
}

pub(crate) struct Readiness<'w, 's, P> {
	resources: FilteredResources<'w, 's>,
	probes: &'s [Arc<Probe>],
	marker: PhantomData<fn() -> P>,
}

impl<P> Readiness<'_, '_, P> {
	pub(crate) fn ready(&self) -> bool {
		self.probes
			.iter()
			.all(|probe| (probe.ready)(&self.resources))
	}
}

pub(crate) struct ReadinessState {
	resources: <FilteredResources<'static, 'static> as SystemParam>::State,
	probes: Vec<Arc<Probe>>,
}

fn probes<P: ParameterTypes>(world: &World) -> Vec<Arc<Probe>> {
	let Some(registry) = world.get_resource::<CatalogReadiness>() else {
		return Vec::new();
	};

	P::types()
		.iter()
		.filter_map(|id| registry.0.get(id).cloned())
		.collect()
}

// SAFETY: The native FilteredResources builder creates the complete access set;
// init_access (or build on 0.16) registers it with Bevy. get_param delegates to
// that same parameter with its original state, World and ticks. Probes receive
// only its checked, read-only view, so cannot read outside the registered set.
unsafe impl<P: ParameterTypes + 'static> SystemParam for Readiness<'_, '_, P> {
	type State = ReadinessState;
	type Item<'w, 's> = Readiness<'w, 's, P>;

	#[cfg(feature = "bevy-0-16")]
	fn init_state(world: &mut World, meta: &mut SystemMeta) -> Self::State {
		let probes = probes::<P>(world);
		let resources =
			FilteredResourcesParamBuilder::new(|builder: &mut FilteredResourcesBuilder| {
				for probe in &probes {
					(probe.access)(builder);
				}
			})
			.build(world, meta);

		ReadinessState { resources, probes }
	}

	#[cfg(not(feature = "bevy-0-16"))]
	fn init_state(world: &mut World) -> Self::State {
		let probes = probes::<P>(world);
		let resources =
			FilteredResourcesParamBuilder::new(|builder: &mut FilteredResourcesBuilder| {
				for probe in &probes {
					(probe.access)(builder);
				}
			})
			.build(world);

		ReadinessState { resources, probes }
	}

	#[cfg(not(feature = "bevy-0-16"))]
	fn init_access(
		state: &Self::State,
		meta: &mut SystemMeta,
		access: &mut crate::bevy::ecs::query::FilteredAccessSet,
		world: &mut World,
	) {
		FilteredResources::init_access(&state.resources, meta, access, world);
	}

	#[cfg(not(feature = "bevy-0-19"))]
	unsafe fn get_param<'w, 's>(
		state: &'s mut Self::State,
		meta: &SystemMeta,
		world: UnsafeWorldCell<'w>,
		tick: Tick,
	) -> Self::Item<'w, 's> {
		// SAFETY: Our caller guarantees the same World and access recorded by the
		// delegated initialization. State and lifetimes are forwarded unchanged.
		let resources =
			unsafe { FilteredResources::get_param(&mut state.resources, meta, world, tick) };

		Readiness {
			resources,
			probes: &state.probes,
			marker: PhantomData,
		}
	}

	#[cfg(feature = "bevy-0-19")]
	unsafe fn get_param<'w, 's>(
		state: &'s mut Self::State,
		meta: &SystemMeta,
		world: UnsafeWorldCell<'w>,
		tick: Tick,
	) -> Result<Self::Item<'w, 's>, crate::bevy::ecs::system::SystemParamValidationError> {
		// SAFETY: Our caller guarantees the same World and access recorded by the
		// delegated initialization. State and lifetimes are forwarded unchanged.
		let resources =
			unsafe { FilteredResources::get_param(&mut state.resources, meta, world, tick) }?;

		Ok(Readiness {
			resources,
			probes: &state.probes,
			marker: PhantomData,
		})
	}
}

// SAFETY: FilteredResources exposes shared reads only; probes cannot access World.
unsafe impl<P: ParameterTypes + 'static> ReadOnlySystemParam for Readiness<'_, '_, P> {}
