use super::{
	engine,
	state::{Ownership, State},
};
use crate::bevy::prelude::{Entity, World};
use crate::bindings::Binding;
use crate::{FluentScope, LocalizationTarget, Localized, TargetUpdatePhase};
use std::any::type_name;

impl<T: LocalizationTarget, S: FluentScope> Binding for Localized<T, S> {
	type Scope = S;

	const SHARED_DEMAND: bool = false;

	fn synchronize_consumers(world: &mut World) {
		let changes = world
			.run_system_cached(changed_selections::<S, T>)
			.expect("valid binding ownership query");

		for (entity, locale) in changes {
			let consumer = crate::demand::acquire_binding::<S>(world, locale);

			if let Some(mut state) = world.get_mut::<State<S, T>>(entity) {
				state.consumer = Some(Ownership {
					locale,
					_token: consumer,
				});
			}
		}
	}

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
		let _ = world.unregister_system_cached(changed_selections::<S, T>);
		engine::unregister::<S, T>(world);
	}
}

/// Changed bindings update demand once; unchanged frames allocate no owner list entries.
fn changed_selections<S: FluentScope, T: LocalizationTarget>(
	bindings: SelectionChanges<'_, '_, S, T>,
) -> Vec<(Entity, Option<<S::Catalog as crate::FluentCatalog>::Locale>)> {
	bindings
		.iter()
		.filter(|(_, binding, state)| {
			state
				.consumer
				.as_ref()
				.is_none_or(|consumer| consumer.locale != binding.locale)
		})
		.map(|(entity, binding, _)| (entity, binding.locale))
		.collect()
}

/// Only changed binding metadata is inspected for demand ownership changes.
type SelectionChanges<'w, 's, S, T> = crate::bevy::prelude::Query<
	'w,
	's,
	(Entity, &'static Localized<T, S>, &'static State<S, T>),
	crate::bevy::prelude::Changed<Localized<T, S>>,
>;
