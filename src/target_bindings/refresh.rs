use super::{engine, state::State};
use crate::bevy::prelude::{Entity, World};
use crate::bindings::Binding;
use crate::{FluentScope, LocalizationTarget, Localized, TargetUpdatePhase};

impl<T: LocalizationTarget, S: FluentScope> Binding for Localized<T, S> {
	type Scope = S;

	const PHASE: TargetUpdatePhase = T::PHASE;

	fn refresh(world: &mut World) {
		engine::refresh::<S, T>(world);
	}

	fn initialize(world: &mut World, entity: Entity) {
		let Ok(mut entity) = world.get_entity_mut(entity) else {
			return;
		};

		if !entity.contains::<Self>() {
			return;
		}

		if !entity.contains::<T>() {
			entity.insert(T::default());
		}

		T::initialize(&mut entity);

		if entity.contains::<Self>() {
			entity.insert(State::<S, T>::default());
		}
	}

	fn retire(world: &mut World, entity: Entity) {
		if let Ok(mut entity) = world.get_entity_mut(entity) {
			entity.remove::<State<S, T>>();
		}
	}

	fn unregister(world: &mut World) {
		engine::unregister::<S, T>(world);
	}
}
