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
use crate::{FluentCatalog, LoadingMode, Localization, ScopeRegistration};
use std::{
	any::TypeId,
	collections::HashMap,
	marker::PhantomData,
	sync::{
		Arc,
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
/// No loading is requested: Lazy applications retain explicit `load` / `unload`.
///
/// Register in a recurring schedule such as `Update`. A skipped `Startup` or
/// `OnEnter` system cannot retry; use [`LocalizationAppExt::add_localized_startup_systems`]
/// for one-time initialization after readiness. Custom derived `SystemParam`s and
/// nested parameter tuples are not inspected; use a direct `Res` parameter for
/// each required catalog. Ordinary missing resources keep Bevy's validation behavior.
/// Install localization plugins before the schedule is first initialized or run,
/// so its inferred resource reads can be registered with the scheduler.
pub fn localized<M>(systems: impl IntoLocalizedSystems<M>) -> ScheduleConfigs<ScheduleSystem> {
	systems.into_localized(false)
}

/// Register native-resource consumers that wait without blocking the frame.
pub trait LocalizationAppExt {
	/// Enable recurring progress snapshots for one installed provider.
	///
	/// Adds tracking systems and initializes `LocalizationProgress<C>` before
	/// Startup once its localization plugin is installed. May be called before or
	/// after adding the plugin, and repeatedly without duplicating systems. The
	/// plugin's Full/Lazy mode is inferred; no Cargo feature is required.
	/// Observe in Update, or after `LocalizationSystems::Progress` in PostUpdate.
	fn add_localization_progress<C: FluentCatalog>(&mut self) -> &mut Self;

	/// Add recurring functions with inferred catalog readiness gates.
	///
	/// Accepts functions and tuples. Use [`localized`] with `add_systems` when
	/// applying Bevy configuration. Built-in startup schedules are rejected because
	/// they cannot retry a skipped system; custom one-shot schedules have the same
	/// limitation and must not be used here.
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
	fn add_localization_progress<C: FluentCatalog>(&mut self) -> &mut Self {
		crate::progress::request::<C>(self);
		self
	}

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
		let system = if once {
			Tracked {
				function: self,
				completed: completed.clone(),
				marker: PhantomData,
			}
			.into_configs()
		} else {
			self.into_configs()
		};

		system.run_if(move |readiness: Readiness<F::Param>| {
			if once && completed.load(Ordering::Relaxed) {
				return false;
			}

			readiness.ready()
		})
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
