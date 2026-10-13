//! Explicitly passive native resource borrowing.
use crate::FluentScope;
use crate::bevy::{
	ecs::{
		self as bevy_ecs,
		change_detection::Tick,
		component::ComponentId,
		query::FilteredAccess,
		system::{ReadOnlySystemParam, SystemMeta, SystemParam, SystemParamValidationError},
		world::unsafe_world_cell::UnsafeWorldCell,
	},
	prelude::*,
};
use crate::compatibility::scheduling::{IS_RESOURCE, SystemAccess};
use std::{any::TypeId, collections::HashMap, marker::PhantomData, ops::Deref};

/// Borrow an available catalog without requesting or retaining its module.
///
/// Available on Bevy 0.19/0.20. Use `Option<PassiveCatalog<Scope>>` to receive `None`
/// immediately when unavailable.
/// Like native `Res`, the borrow lasts only for the current system invocation.
/// A plain `Option<Res<Scope>>` has the same scheduler metadata as required `Res`
/// and is conservatively treated as a consumer by automatic discovery.
/// Only localization scopes implementing [`FluentScope`] can be borrowed.
/// After matching `Some(catalog)`, dereferencing exposes the scope's own methods.
///
/// Ordinary Bevy resources are not localization scopes:
///
/// ```compile_fail
/// use bevy_fluent_typed::{PassiveCatalog, bevy::{ecs as bevy_ecs, prelude::*}};
///
/// #[derive(Resource, Clone)]
/// struct Settings;
///
/// fn observe(_: Option<PassiveCatalog<Settings>>) {}
/// ```
pub struct PassiveCatalog<'w, T: FluentScope>(Res<'w, T>);

impl<T: FluentScope> Deref for PassiveCatalog<'_, T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<'w, T: FluentScope> PassiveCatalog<'w, T> {
	/// Obtain the borrowed value with the world's lifetime.
	pub fn into_inner(self) -> &'w T {
		self.0.into_inner()
	}
}

#[derive(Component)]
pub(crate) struct PassiveCatalogMarker<T: FluentScope>(PhantomData<fn() -> T>);

#[derive(Resource, Default)]
pub(crate) struct PassiveCatalogMarkers(pub(crate) HashMap<TypeId, ComponentId>);

// SAFETY: The access retains native Res's resource filter and adds only a raw
// read permission for a private marker. It does not add a phantom With filter
// that could hide real conflicts. Native Res performs the checked fetch.
unsafe impl<T: FluentScope> SystemParam for PassiveCatalog<'_, T> {
	type State = (ComponentId, ComponentId);
	type Item<'w, 's> = PassiveCatalog<'w, T>;

	fn init_state(world: &mut World) -> Self::State {
		let resource = Res::<T>::init_state(world);
		let marker = world.register_component::<PassiveCatalogMarker<T>>();
		world.init_resource::<PassiveCatalogMarkers>();
		world
			.resource_mut::<PassiveCatalogMarkers>()
			.0
			.insert(TypeId::of::<T>(), marker);
		(resource, marker)
	}

	fn init_access(
		state: &Self::State,
		_: &mut SystemMeta,
		access: &mut SystemAccess,
		_: &mut World,
	) {
		let mut filter = FilteredAccess::default();
		filter.add_read(state.0);
		filter.and_with(IS_RESOURCE);
		filter.access_mut().add_read(state.1);
		#[cfg(feature = "bevy-0-20")]
		assert!(
			access.try_add(filter).is_ok(),
			"PassiveCatalog conflicts with a mutable resource parameter"
		);
		#[cfg(feature = "bevy-0-19")]
		{
			assert!(
				access.get_conflicts_single(&filter).is_empty(),
				"PassiveCatalog conflicts with a mutable resource parameter"
			);
			access.add(filter);
		}
	}

	unsafe fn get_param<'w, 's>(
		state: &'s mut Self::State,
		meta: &SystemMeta,
		world: UnsafeWorldCell<'w>,
		tick: Tick,
	) -> Result<Self::Item<'w, 's>, SystemParamValidationError> {
		// SAFETY: Native Res receives its original state and the access recorded above.
		unsafe { Res::<T>::get_param(&mut state.0, meta, world, tick) }.map(PassiveCatalog)
	}
}

// SAFETY: Delegated Res fetches are shared reads only.
unsafe impl<T: FluentScope> ReadOnlySystemParam for PassiveCatalog<'_, T> {}
