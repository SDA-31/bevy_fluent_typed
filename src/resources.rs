//! Publish a whole typed resource tree without idle-frame change notifications.
use crate::{FluentCatalog, Localization};
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Resource)]
struct Published<C: FluentCatalog>(Arc<C>);

/// Publish the complete active tree under exclusive World access.
/// Arc identity suppresses redundant resource changes; Localization must exist.
pub(crate) fn synchronize<C: FluentCatalog>(world: &mut World) {
	let snapshot = world.resource::<Localization<C>>().snapshot();

	if world
		.get_resource::<Published<C>>()
		.is_some_and(|published| Arc::ptr_eq(&snapshot, &published.0))
	{
		return;
	}

	// Other systems cannot observe a partly published resource tree.
	snapshot.publish_resources(world);
	world.insert_resource(Published(snapshot));
}
