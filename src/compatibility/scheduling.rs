//! Public scheduling metadata differs between Bevy 0.19 and 0.20.
#[cfg(feature = "bevy-0-20")]
pub(crate) use crate::bevy::ecs::{component::IS_RESOURCE, system::SystemAccess};
#[cfg(feature = "bevy-0-19")]
pub(crate) use crate::bevy::ecs::{
	query::FilteredAccessSet as SystemAccess, resource::IS_RESOURCE,
};
