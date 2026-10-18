use super::{engine, state::State};
use crate::bevy::prelude::{Entity, World};
use crate::bindings::Binding;
use crate::{FluentScope, LocalizationTarget, Localized, TargetUpdatePhase};
use std::any::type_name;

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
			tracing::warn!(
				entity = ?entity.id(),
				target_component = type_name::<T>(),
				scope = type_name::<S>(),
				"removing Localized: target component is missing; spawn the target alongside its binding"
			);
			entity.remove::<Self>();
			return;
		}

		entity.insert(State::<S, T>::default());
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
