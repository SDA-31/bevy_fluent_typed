use super::{
	deferred::{self, Deferred},
	parallel::{MIN_FORMATTING_BINDINGS, MIN_REPAIR_BINDINGS, should_parallelize},
	state::State,
};
use crate::bevy::{
	ecs::system::SystemChangeTick,
	prelude::{
		Commands, DetectChanges, DetectChangesMut, Entity, Local, ParamSet, Query, Res, World,
	},
	utils::Parallel,
};
use crate::message::MsgIdentity;
use crate::{
	CachePolicy, FluentScope, LocalizationTarget, LocalizationUpdate, Localized, TargetItem,
	UpdateResult, cache_policy,
};

/// The adapter's declared query, extended with its binding and tracking state.
type Bindings<'w, 's, S, T> = Query<
	'w,
	's,
	(
		Entity,
		&'static Localized<T, S>,
		<T as LocalizationTarget>::Data,
		&'static mut State<S, T>,
	),
	<T as LocalizationTarget>::Filter,
>;

/// Potentially overlapping catalog and target borrows, accessed through a ParamSet.
type Parameters<'w, 's, S, T> = (Option<Res<'w, S>>, Bindings<'w, 's, S, T>);

/// Work retained per worker until all parallel target callbacks have finished.
///
/// Structural updates run later on the World. Superseded weak message identities
/// are also retired together after the parallel pass.
struct Work<S: FluentScope> {
	deferred: Vec<Deferred<S>>,
	retired: Vec<MsgIdentity<S>>,
}

impl<S: FluentScope> Default for Work<S> {
	fn default() -> Self {
		Self {
			deferred: Vec::new(),
			retired: Vec::new(),
		}
	}
}

pub(super) fn refresh<S: FluentScope, T: LocalizationTarget>(world: &mut World) {
	match cache_policy::for_scope::<S>(world) {
		CachePolicy::Rendered => world.run_system_cached(refresh_bindings::<S, T, true>),
		CachePolicy::None => world.run_system_cached(refresh_bindings::<S, T, false>),
	}
	.expect("valid localization target query");
}

pub(super) fn unregister<S: FluentScope, T: LocalizationTarget>(world: &mut World) {
	let _ = world.unregister_system_cached(refresh_bindings::<S, T, true>);
	let _ = world.unregister_system_cached(refresh_bindings::<S, T, false>);
}

/// Catalog snapshot and invalidation clocks shared by one complete query pass.
struct Refresh<'a, S> {
	catalog: &'a Option<S>,
	changed: bool,
	previous: u64,
	current: u64,
	this_run: u32,
}

fn needs_update<S: FluentScope, T: LocalizationTarget>(
	binding: &Localized<T, S>,
	data: &TargetItem<'_, '_, T>,
	state: &mut State<S, T>,
	frame: &Refresh<S>,
) -> bool {
	let invalidated = frame.changed || !state.current(&binding.message, frame.previous);
	let changed = invalidated || state.pending;
	let update = LocalizationUpdate::cached(changed, "").observe(state.observed(), frame.this_run);
	let needed = T::needs_update(data, &update);

	// A predicate can suspend work across catalog changes. Keep the old visit
	// until that invalidation is handled, so a later pass uses the latest value.
	if !needed && !invalidated {
		state.pass = frame.current;
	}

	needed
}

fn update<S: FluentScope, T: LocalizationTarget, const CACHE: bool>(
	entity: Entity,
	binding: &Localized<T, S>,
	data: TargetItem<'_, '_, T>,
	state: &mut State<S, T>,
	frame: &Refresh<S>,
) -> (Option<MsgIdentity<S>>, Option<Deferred<S>>) {
	let Refresh {
		catalog,
		changed,
		previous,
		current,
		this_run,
	} = *frame;
	let invalidated = changed || !state.current(&binding.message, previous);
	state.pending |= invalidated;
	let render = || {
		catalog
			.as_ref()
			.map_or_else(String::new, |scope| binding.message.render(scope))
	};

	let fresh = (CACHE && (invalidated || state.text.is_none())).then(render);

	let observed = state.observed();
	let mut context = if CACHE {
		LocalizationUpdate::cached_update(
			state.pending,
			state.text.get_or_insert_with(Box::default),
			fresh,
		)
	} else {
		LocalizationUpdate::lazy(state.pending, &render)
	}
	.observe(observed, this_run);
	let result = T::update(data, &mut context);
	context.prepare_cache();
	let read = context.was_read();
	let deferred = match result {
		UpdateResult::Done => None,
		UpdateResult::Deferred => Some(Deferred {
			entity,
			message: binding.message.identity(),
			changed: state.pending,
			text: context.into_owned(),
			read,
			applied: false,
		}),
	};

	if read && matches!(result, UpdateResult::Done) {
		state.pending = false;
	}

	if matches!(result, UpdateResult::Done) {
		state.confirm(this_run);
	}

	state.pass = current;
	let retired = if state
		.message
		.as_ref()
		.is_some_and(|identity| identity.matches(&binding.message))
	{
		None
	} else {
		state.message.replace(binding.message.identity())
	};
	(retired, deferred)
}

/// Execution path chosen without allocating another collection of binding rows.
enum RefreshPlan {
	/// Selection visited every row and found no target that needs an update.
	Idle,
	/// Update only the selected entities retained in the reusable pending buffer.
	Pending,
	/// Inspect and update the full query on the calling thread.
	Sequential,
	/// Run the native parallel query; selection has already stamped clean rows
	/// when `selected` is true, allowing the workers to skip them.
	Parallel { selected: bool },
}

fn refresh_bindings<S: FluentScope, T: LocalizationTarget, const CACHE: bool>(
	mut params: ParamSet<Parameters<S, T>>,
	mut work: Local<Parallel<Work<S>>>,
	mut pending: Local<Vec<Entity>>,
	mut pass: Local<u64>,
	mut was_ready: Local<bool>,
	ticks: SystemChangeTick,
	mut commands: Commands,
) {
	// Resource entities can also occur in user Data. End that resource borrow first.
	let (catalog, changed) = {
		let resource = params.p0();
		let changed = *was_ready != resource.is_some()
			|| resource.as_ref().is_some_and(|value| value.is_changed());
		*was_ready = resource.is_some();
		(resource.as_deref().cloned(), changed)
	};

	let mut bindings = params.p1();
	// Only a scheduling estimate is needed. Query's native upper bound avoids
	// traversing every row just to decide whether another traversal may parallelize.
	let count = bindings.iter().size_hint().1.unwrap_or(0);
	let previous = *pass;
	*pass = pass.wrapping_add(1);
	let current = *pass;
	let this_run = ticks.this_run().get();

	let frame = Refresh {
		catalog: &catalog,
		changed,
		previous,
		current,
		this_run,
	};

	let deferred = match plan_refresh::<S, T, CACHE>(&mut bindings, &mut pending, &frame, count) {
		RefreshPlan::Idle => return,
		RefreshPlan::Pending => refresh_pending::<S, T, CACHE>(&mut bindings, &mut pending, &frame),
		RefreshPlan::Sequential => refresh_sequential::<S, T, CACHE>(&mut bindings, &frame),
		RefreshPlan::Parallel { selected } => {
			refresh_parallel::<S, T, CACHE>(&mut bindings, &mut work, &frame, selected)
		}
	};

	deferred::queue::<S, T>(&mut commands, deferred, catalog);
}

fn plan_refresh<S: FluentScope, T: LocalizationTarget, const CACHE: bool>(
	bindings: &mut Bindings<'_, '_, S, T>,
	pending: &mut Vec<Entity>,
	frame: &Refresh<S>,
	count: usize,
) -> RefreshPlan {
	if !should_parallelize(count, MIN_FORMATTING_BINDINGS.min(MIN_REPAIR_BINDINGS)) {
		return RefreshPlan::Sequential;
	}

	if !frame.changed {
		return select_pending::<S, T, CACHE>(bindings, pending, frame);
	}

	let minimum = if frame.catalog.is_some() {
		MIN_FORMATTING_BINDINGS
	} else {
		MIN_REPAIR_BINDINGS
	};

	if count < minimum {
		return RefreshPlan::Sequential;
	}

	RefreshPlan::Parallel { selected: false }
}

fn select_pending<S: FluentScope, T: LocalizationTarget, const CACHE: bool>(
	bindings: &mut Bindings<'_, '_, S, T>,
	pending: &mut Vec<Entity>,
	frame: &Refresh<S>,
) -> RefreshPlan {
	pending.clear();
	let mut dirty = 0;
	let mut formatting = 0;
	let mut parallel = false;

	for (entity, binding, data, mut state) in bindings {
		let state = state.bypass_change_detection();

		if !needs_update(binding, &data, state, frame) {
			continue;
		}

		dirty += 1;
		formatting += usize::from(!state.current(&binding.message, frame.previous) || !CACHE);

		if parallel {
			continue;
		}

		parallel = formatting >= MIN_FORMATTING_BINDINGS || dirty >= MIN_REPAIR_BINDINGS;

		if parallel {
			pending.clear();
			continue;
		}

		pending.push(entity);
	}

	if dirty == 0 {
		return RefreshPlan::Idle;
	}

	if !parallel {
		return RefreshPlan::Pending;
	}

	RefreshPlan::Parallel { selected: true }
}

fn refresh_pending<S: FluentScope, T: LocalizationTarget, const CACHE: bool>(
	bindings: &mut Bindings<'_, '_, S, T>,
	pending: &mut Vec<Entity>,
	frame: &Refresh<S>,
) -> Vec<Deferred<S>> {
	let mut deferred = Vec::new();

	for entity in pending.drain(..) {
		let Ok((_, binding, data, mut state)) = bindings.get_mut(entity) else {
			continue;
		};
		let (retired, item) = update::<S, T, CACHE>(
			entity,
			binding,
			data,
			state.bypass_change_detection(),
			frame,
		);

		drop(retired);
		deferred.extend(item);
	}

	deferred
}

fn refresh_sequential<S: FluentScope, T: LocalizationTarget, const CACHE: bool>(
	bindings: &mut Bindings<'_, '_, S, T>,
	frame: &Refresh<S>,
) -> Vec<Deferred<S>> {
	let mut deferred = Vec::new();

	for (entity, binding, data, mut state) in bindings {
		let state = state.bypass_change_detection();

		if !needs_update(binding, &data, state, frame) {
			continue;
		}

		let (retired, item) = update::<S, T, CACHE>(entity, binding, data, state, frame);

		drop(retired);
		deferred.extend(item);
	}

	deferred
}

fn refresh_parallel<S: FluentScope, T: LocalizationTarget, const CACHE: bool>(
	bindings: &mut Bindings<'_, '_, S, T>,
	work: &mut Parallel<Work<S>>,
	frame: &Refresh<S>,
	selected: bool,
) -> Vec<Deferred<S>> {
	bindings.par_iter_mut().for_each_init(
		|| work.borrow_local_mut(),
		|work, (entity, binding, data, mut state)| {
			let state = state.bypass_change_detection();

			if selected && state.pass == frame.current {
				return;
			}

			if !needs_update(binding, &data, state, frame) {
				return;
			}

			let (retired, deferred) = update::<S, T, CACHE>(entity, binding, data, state, frame);
			work.retired.extend(retired);
			work.deferred.extend(deferred);
		},
	);

	let mut deferred = Vec::new();

	for work in work.iter_mut() {
		work.retired.clear();
		deferred.append(&mut work.deferred);
	}

	deferred
}
