use super::state::State;
use crate::bevy::{
	ecs::query::QueryState,
	prelude::{Commands, World},
};
use crate::message::MsgIdentity;
use crate::{FluentScope, LocalizationTarget, LocalizationUpdate, Localized};

/// Structural continuation of an adapter update, validated again before application.
///
/// The weak message identity rejects obsolete work without retaining captures.
/// Application and confirmation are separate passes so later hooks cannot make an
/// earlier row's observed change tick acknowledge an unapplied or overwritten value.
pub(super) struct Deferred<S: FluentScope> {
	pub(super) entity: crate::bevy::prelude::Entity,
	pub(super) message: MsgIdentity<S>,
	/// A prepared value, or None while formatting remains lazy.
	pub(super) text: Option<String>,
	pub(super) changed: bool,
	/// Whether either the query callback or its continuation consumed the value.
	pub(super) read: bool,
	/// Whether this continuation ran, rather than being excluded by a later filter.
	pub(super) applied: bool,
}

pub(super) fn queue<S: FluentScope, T: LocalizationTarget>(
	commands: &mut Commands,
	mut batch: Vec<Deferred<S>>,
	catalog: Option<S>,
) {
	if batch.is_empty() {
		return;
	}

	commands.queue(move |world: &mut World| {
		let mut query = world.query_filtered::<T::Data, T::Filter>();

		for item in &mut batch {
			apply_one::<S, T>(world, &mut query, item, &catalog);
		}

		// Earlier rows can be changed by hooks belonging to later rows. Confirm
		// only after all structural callbacks, using the adapter's same Query.
		let observed = world.change_tick().get();

		for item in batch {
			confirm_one::<S, T>(world, &mut query, item, observed);
		}

		// Later application writes must remain distinguishable from this batch.
		world.increment_change_tick();
	});
}

fn apply_one<S: FluentScope, T: LocalizationTarget>(
	world: &mut World,
	query: &mut QueryState<T::Data, T::Filter>,
	item: &mut Deferred<S>,
	catalog: &Option<S>,
) {
	let this_run = world.change_tick().get();

	if query.get(world, item.entity).is_err() {
		return;
	}

	let Ok(mut entity) = world.get_entity_mut(item.entity) else {
		return;
	};
	let Some(binding) = entity.get::<Localized<T, S>>() else {
		return;
	};

	if !item.message.matches(&binding.message) {
		return;
	}

	let message = binding.message.clone();
	let observed = entity.get::<State<S, T>>().and_then(State::observed);
	let render = || {
		catalog
			.as_ref()
			.map_or_else(String::new, |scope| message.render(scope))
	};
	let mut update = if let Some(text) = item.text.take() {
		LocalizationUpdate::owned(item.changed, text)
	} else {
		LocalizationUpdate::lazy(item.changed, &render)
	}
	.observe(observed, this_run);
	T::apply(&mut entity, &mut update);
	item.applied = true;
	item.read |= update.was_read();
	item.text = update.into_owned();
}

fn confirm_one<S: FluentScope, T: LocalizationTarget>(
	world: &mut World,
	query: &mut QueryState<T::Data, T::Filter>,
	item: Deferred<S>,
	observed: u32,
) {
	let Some(binding) = world.get::<Localized<T, S>>(item.entity) else {
		return;
	};

	if !item.message.matches(&binding.message) {
		return;
	}

	let confirmed = item.applied
		&& item.text.as_deref().is_some_and(|text| {
			query
				.get(world, item.entity)
				.is_ok_and(|data| T::matches(data, text))
		});
	let Some(mut state) = world.get_mut::<State<S, T>>(item.entity) else {
		return;
	};

	if !item.applied {
		// A filter can exclude this row and return within one batch.
		// Its query visit cannot acknowledge a write that never ran.
		state.pending = true;
		return;
	}

	state.pending &= !item.read;

	if confirmed {
		state.confirm(observed);
	}
}
