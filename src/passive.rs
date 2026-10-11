//! Explicitly passive native resource borrowing.
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
/// Use `Option<PassiveRes<Scope>>` to receive `None` immediately when unavailable.
/// Like native `Res`, the borrow lasts only for the current system invocation.
/// A plain `Option<Res<Scope>>` has the same scheduler metadata as required `Res`
/// and is conservatively treated as a consumer by automatic discovery.
pub struct PassiveRes<'w, T: Resource>(Res<'w, T>);

impl<T: Resource> Deref for PassiveRes<'_, T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<'w, T: Resource> PassiveRes<'w, T> {
	/// Obtain the borrowed value with the world's lifetime.
	pub fn into_inner(self) -> &'w T {
		self.0.into_inner()
	}
}

#[derive(Component)]
pub(crate) struct PassiveMarker<T: Resource>(PhantomData<fn() -> T>);

#[derive(Resource, Default)]
pub(crate) struct PassiveMarkers(pub(crate) HashMap<TypeId, ComponentId>);

// SAFETY: The access retains native Res's resource filter and adds only a raw
// read permission for a private marker. It does not add a phantom With filter
// that could hide real conflicts. Native Res performs the checked fetch.
unsafe impl<T: Resource> SystemParam for PassiveRes<'_, T> {
	type State = (ComponentId, ComponentId);
	type Item<'w, 's> = PassiveRes<'w, T>;

	fn init_state(world: &mut World) -> Self::State {
		let resource = Res::<T>::init_state(world);
		let marker = world.register_component::<PassiveMarker<T>>();
		world.init_resource::<PassiveMarkers>();
		world
			.resource_mut::<PassiveMarkers>()
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
			"PassiveRes conflicts with a mutable resource parameter"
		);
		#[cfg(feature = "bevy-0-19")]
		{
			assert!(
				access.get_conflicts_single(&filter).is_empty(),
				"PassiveRes conflicts with a mutable resource parameter"
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
		unsafe { Res::<T>::get_param(&mut state.0, meta, world, tick) }.map(PassiveRes)
	}
}

// SAFETY: Delegated Res fetches are shared reads only.
unsafe impl<T: Resource> ReadOnlySystemParam for PassiveRes<'_, T> {}
