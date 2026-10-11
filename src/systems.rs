//! Readiness gates inferred from native, top-level `Res<Scope>` parameters.
use crate::bevy::{
	ecs::{
		self as bevy_ecs,
		schedule::{ScheduleConfigs, ScheduleLabel},
		system::{ScheduleSystem, SystemParam, SystemParamFunction, SystemParamItem},
	},
	prelude::*,
};
use crate::compatibility::readiness::{Probe, Readiness};
use crate::demand::Consumers;
use crate::{FluentCatalog, LoadingMode, Localization, ScopeRegistration};
use std::{
	any::TypeId,
	collections::HashMap,
	marker::PhantomData,
	sync::{
		Arc, OnceLock,
		atomic::{AtomicBool, Ordering},
	},
};

#[derive(Resource, Default)]
pub(crate) struct CatalogReadiness(pub(crate) HashMap<TypeId, Arc<Probe>>);

impl CatalogReadiness {
	pub(crate) fn register<C: FluentCatalog, M: LoadingMode>(
		world: &mut World,
		scope: &ScopeRegistration<C>,
	) {
		let id = scope.id;
		let exists = scope.ready_exists;
		let access = scope.ready_access;
		world.init_resource::<Self>();
		world.resource_mut::<Self>().0.insert(
			scope.parameter,
			Arc::new(Probe {
				#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
				scope: id,
				consumer: M::AUTOMATIC.then_some(scope.consumer),
				access: Box::new(move |builder| {
					builder.add_read::<Localization<C, M>>();
					access(builder);
				}),
				ready: Box::new(move |resources| {
					let Ok(localization) = resources.get::<Localization<C, M>>() else {
						return false;
					};

					localization.store.values.get(&id).is_some_and(|value| {
						localization.published.get(&id) == Some(&value.revision)
							&& exists(resources)
					})
				}),
			}),
		);
	}
}

/// Wait for the registered catalogs required by a function or tuple of functions.
///
/// Functions keep native `Res<Scope>` parameters. Each function waits independently
/// for its direct, required catalog parameters.
/// Apply Bevy configuration (`chain`, `run_if`, `in_set`, etc.) to the returned value.
/// In default Lazy, each direct required catalog is requested when Bevy initializes
/// the system. Recurring systems retain demand while their state exists, including
/// when `run_if` is false. Explicit Manual still needs manual `load` / `unload`;
/// this helper's optional resource parameters do not create demand. On Bevy
/// 0.19/0.20 ordinary `add_systems` already discovers native resource access;
/// these helpers remain available for older backends and compatibility.
///
/// Register this helper in a recurring schedule such as `Update`. For ordinary
/// `Startup` on Bevy 0.19/0.20, prefer native `add_systems` with Deferred startup.
/// On older backends, [`LocalizationAppExt::add_localized_startup_systems`] provides
/// independent one-time initialization in Update. `OnEnter` and custom one-shot
/// schedules do not retry skipped consumers. Custom derived `SystemParam`s and
/// nested parameter tuples are not inspected; use a direct `Res` parameter for
/// each required catalog. Ordinary missing resources keep Bevy's validation behavior.
/// Install localization plugins before the schedule is first initialized or run,
/// so its inferred resource reads can be registered with the scheduler.
pub fn localized<M>(systems: impl IntoLocalizedSystems<M>) -> ScheduleConfigs<ScheduleSystem> {
	systems.into_localized(false)
}

/// Explicit resource-readiness registration for older backends and existing code.
///
/// On Bevy 0.19/0.20, ordinary `App::add_systems` already discovers registered
/// catalog reads. Its default Deferred startup preserves native startup ordering;
/// this extension's one-time helper instead keeps its independent Update semantics.
pub trait LocalizationAppExt {
	/// Add recurring functions with inferred catalog ownership and readiness gates.
	/// Lazy owns required scopes for the system-state lifetime, even while skipped.
	///
	/// Accepts functions and tuples. Use [`localized`] with `add_systems` when
	/// applying Bevy configuration. This recurring helper rejects built-in startup
	/// schedules. On Bevy 0.19/0.20 use ordinary `add_systems` for Deferred startup;
	/// custom one-shot schedules do not retry waiting consumers.
	fn add_localized_systems<M>(
		&mut self,
		schedule: impl ScheduleLabel,
		systems: impl IntoLocalizedSystems<M>,
	) -> &mut Self;

	/// Run each function once in `Update`, after its required catalogs are ready.
	///
	/// Completion is recorded only after the function body returns. This allows
	/// deferred commands and preserves returned errors. An invoked function that
	/// returns an error has still run once; normal Bevy error handling applies.
	/// In Lazy, demand is released after the body returns; bindings inserted by
	/// deferred commands establish their own ownership before module release.
	/// This is independent deferred initialization, not Startup ordering. The
	/// function is wrapped: `before(original_function)` / `after(original_function)`
	/// do not target that wrapper. Combine dependent steps into one function, or
	/// use recurring [`localized`] systems with application-owned initialization state.
	fn add_localized_startup_systems<M>(
		&mut self,
		systems: impl IntoLocalizedSystems<M>,
	) -> &mut Self;
}

impl LocalizationAppExt for App {
	fn add_localized_systems<M>(
		&mut self,
		schedule: impl ScheduleLabel,
		systems: impl IntoLocalizedSystems<M>,
	) -> &mut Self {
		let schedule = schedule.intern();
		assert!(
			![PreStartup.intern(), Startup.intern(), PostStartup.intern()].contains(&schedule),
			"startup schedules cannot retry; use add_localized_startup_systems"
		);
		self.add_systems(schedule, localized(systems))
	}

	fn add_localized_startup_systems<M>(
		&mut self,
		systems: impl IntoLocalizedSystems<M>,
	) -> &mut Self {
		self.add_systems(Update, systems.into_localized(true))
	}
}

/// Functions and tuples accepted by [`localized`] and [`LocalizationAppExt`].
///
/// Implemented automatically for ordinary Bevy function systems. The marker is
/// inferred; applications do not implement this trait or specify it explicitly.
pub trait IntoLocalizedSystems<M> {
	/// Build independent readiness gates, optionally completing after one invocation.
	#[doc(hidden)]
	fn into_localized(self, once: bool) -> ScheduleConfigs<ScheduleSystem>;
}

// Distinct marker shapes avoid overlap between functions and tuples of functions.
#[doc(hidden)]
pub struct FunctionMarker;
#[doc(hidden)]
pub struct TupleMarker;

struct Tracked<F, M> {
	function: F,
	completed: Arc<AtomicBool>,
	consumers: Arc<OnceLock<Consumers>>,
	marker: PhantomData<fn() -> M>,
}

impl<F, M> SystemParamFunction<(FunctionMarker, M)> for Tracked<F, M>
where
	M: 'static,
	F: SystemParamFunction<M, In = ()>,
{
	type In = ();
	type Out = F::Out;
	type Param = F::Param;

	fn run(&mut self, input: (), params: SystemParamItem<Self::Param>) -> Self::Out {
		let result = self.function.run(input, params);
		self.completed.store(true, Ordering::Relaxed);

		if let Some(consumers) = self.consumers.get() {
			for consumer in consumers.iter() {
				consumer.release();
			}
		}

		result
	}
}

impl<F, M, S, T> IntoLocalizedSystems<(FunctionMarker, M, S, T)> for F
where
	M: 'static,
	F: SystemParamFunction<M, In = ()> + IntoScheduleConfigs<ScheduleSystem, S>,
	F::Param: ParameterTypes,
	Tracked<F, M>: IntoScheduleConfigs<ScheduleSystem, T>,
{
	fn into_localized(self, once: bool) -> ScheduleConfigs<ScheduleSystem> {
		let completed = Arc::new(AtomicBool::new(false));
		let consumers = Arc::new(OnceLock::new());
		let system = if once {
			Tracked {
				function: self,
				completed: completed.clone(),
				consumers: consumers.clone(),
				marker: PhantomData,
			}
			.into_configs()
		} else {
			self.into_configs()
		};

		system
			.run_if(move |readiness: Readiness<F::Param>| {
				if once && completed.load(Ordering::Relaxed) {
					return false;
				}

				if once {
					consumers.get_or_init(|| readiness.consumers.clone());
				}

				readiness.ready()
			})
			.in_set(ExplicitReadiness)
	}
}

pub(crate) trait ParameterTypes {
	fn types() -> Vec<TypeId>;
}

impl ParameterTypes for () {
	fn types() -> Vec<TypeId> {
		Vec::new()
	}
}

macro_rules! parameters {
	($($param:ident),+) => {
		impl<$($param: SystemParam + 'static),+> ParameterTypes for ($($param,)+) {
			fn types() -> Vec<TypeId> {
				vec![$(TypeId::of::<$param>()),+]
			}
		}
	};
}

macro_rules! functions {
	($(($function:ident, $marker:ident)),+) => {
		impl<$($function, $marker),+> IntoLocalizedSystems<(TupleMarker, $($marker,)+)>
			for ($($function,)+)
		where
			$($function: IntoLocalizedSystems<$marker>,)+
		{
			#[allow(non_snake_case)]
			fn into_localized(self, once: bool) -> ScheduleConfigs<ScheduleSystem> {
				let ($($function,)+) = self;
				($($function.into_localized(once),)+).into_configs()
			}
		}
	};
}

macro_rules! tuples {
	(@prefix [$($prefix:tt)*]) => {};
	(@prefix [$($prefix:tt)*] ($param:ident, $marker:ident) $(($tail:ident, $tail_marker:ident))*) => {
		tuples!(@impl $($prefix)* ($param, $marker));
		tuples!(@prefix [$($prefix)* ($param, $marker)] $(($tail, $tail_marker))*);
	};
	(@impl $(($param:ident, $marker:ident))+) => {
		parameters!($($param),+);
		functions!($(($param, $marker)),+);
	};
}

tuples!(@prefix []
	(A, MA) (B, MB) (C, MC) (D, MD) (E, ME) (F, MF) (G, MG) (H, MH)
	(I, MI) (J, MJ) (K, MK) (L, ML) (N, MN) (O, MO) (P, MP) (Q, MQ)
);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ExplicitReadiness;
