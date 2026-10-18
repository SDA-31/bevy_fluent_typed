//! Registered scope probes for native resource-access discovery.
#![cfg_attr(
	feature = "bevy-0-20",
	allow(
		deprecated,
		reason = "Native readiness uses Bevy's retained filtered resource view."
	)
)]

use crate::bevy::{
	ecs::{
		self as bevy_ecs,
		world::{FilteredResources, FilteredResourcesBuilder},
	},
	prelude::*,
};
use crate::{FluentCatalog, LoadingMode, Localization, ScopeRegistration, demand::Consumer};
use std::{any::TypeId, collections::HashMap, sync::Arc};

/// Type-erased checked resource read and publication-revision readiness test.
/// Only automatic loading modes supply a consumer-acquisition callback.
pub(super) struct Probe {
	pub(super) scope: TypeId,
	pub(super) access: Box<dyn Fn(&mut FilteredResourcesBuilder) + Send + Sync>,
	pub(super) ready: Box<dyn Fn(&FilteredResources) -> bool + Send + Sync>,
	pub(super) consumer: Option<fn(&mut World) -> Arc<Consumer>>,
}

/// World-owned scope index consulted while native system resource access is discovered.
#[derive(Resource, Default)]
pub(crate) struct CatalogReadiness(pub(super) HashMap<TypeId, Arc<Probe>>);

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
			id,
			Arc::new(Probe {
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
