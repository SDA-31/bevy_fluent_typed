use crate::bevy::prelude::World;
use crate::compatibility::scheduling::{IS_RESOURCE, SystemAccess};
use crate::{compatibility::readiness::Probe, passive::PassiveMarkers, systems::CatalogReadiness};
use std::sync::Arc;

pub(super) fn probes(world: &World, access: &SystemAccess) -> Vec<Arc<Probe>> {
	#[cfg(feature = "bevy-0-20")]
	let access = access.to_filtered_access_set();
	let Some(registry) = world.get_resource::<CatalogReadiness>() else {
		return Vec::new();
	};
	let markers = world.get_resource::<PassiveMarkers>();
	registry
		.0
		.values()
		.filter(|probe| {
			let Some(id) = world.components().get_id(probe.scope) else {
				return false;
			};
			let marker = markers.and_then(|markers| markers.0.get(&probe.scope));
			access.filtered_accesses().iter().any(|filter| {
				filter.access().has_read(id)
					&& filter
						.filter_sets()
						.iter()
						.all(|clause| clause.with().contains(IS_RESOURCE))
					&& marker.is_none_or(|marker| !filter.access().has_read(*marker))
			})
		})
		.cloned()
		.collect()
}
