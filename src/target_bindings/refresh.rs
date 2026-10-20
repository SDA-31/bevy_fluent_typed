use super::{engine, state::State};
use crate::bevy::{
	ecs::{entity_disabling::DefaultQueryFilters, query::QueryState},
	prelude::{Changed, Entity, FromWorld, Local, Mut, World},
};
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
				state.consumer = Some(consumer);
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
	world: &mut World,
	mut bindings: Local<SelectionChanges<S, T>>,
) -> Vec<(Entity, Option<<S::Catalog as crate::FluentCatalog>::Locale>)> {
	bindings
		.query
		.iter(world)
		.filter(|(_, binding, state)| {
			state
				.consumer
				.as_ref()
				.is_none_or(|consumer| !consumer.matches_locale::<S::Catalog>(binding.locale))
		})
		.map(|(entity, binding, _)| (entity, binding.locale))
		.collect()
}

/// Only binding metadata is needed to synchronize catalog ownership.
type SelectionQuery<S, T> =
	QueryState<(Entity, &'static Localized<T, S>, &'static State<S, T>), Changed<Localized<T, S>>>;

/// Ownership follows changed bindings even while target queries exclude them.
///
/// Build this cache once without default disabling filters, including custom
/// application filters. Target refresh queries retain their ordinary filters.
struct SelectionChanges<S: FluentScope, T: LocalizationTarget> {
	query: SelectionQuery<S, T>,
}

impl<S: FluentScope, T: LocalizationTarget> FromWorld for SelectionChanges<S, T> {
	fn from_world(world: &mut World) -> Self {
		let build = |world: &mut World| world.query_filtered();
		let query = if world.contains_resource::<DefaultQueryFilters>() {
			world.resource_scope(|world, _: Mut<DefaultQueryFilters>| build(world))
		} else {
			build(world)
		};

		Self { query }
	}
}
