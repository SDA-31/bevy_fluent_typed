//! Provider-owned policy for retaining formatted binding output.
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{Resource, World},
};
use crate::{FluentCatalog, FluentScope};
use std::marker::PhantomData;

/// Whether automatic bindings retain an additional copy of formatted text.
///
/// This does not change catalog loading or the text owned by target components.
/// Configure the complete provider through [`crate::LocalizationPlugin::with_cache_policy`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CachePolicy {
	/// Retain the last result and reuse it when a target needs repair.
	#[default]
	Rendered,
	/// Retain tracking metadata only; format again when output is needed.
	/// Native texts skip unchanged targets. Custom adapters format only when they
	/// request `LocalizationUpdate::text`, at most once per invocation and its deferred continuation. Adapters that
	/// always request text still format
	/// every refresh; use native component changes to skip unnecessary reads.
	None,
}

#[derive(Resource)]
pub(crate) struct ProviderCachePolicy<C: FluentCatalog> {
	pub(crate) policy: CachePolicy,
	marker: PhantomData<fn() -> C>,
}

impl<C: FluentCatalog> ProviderCachePolicy<C> {
	pub(crate) fn new(policy: CachePolicy) -> Self {
		Self {
			policy,
			marker: PhantomData,
		}
	}
}

pub(crate) fn for_scope<S: FluentScope>(world: &World) -> CachePolicy {
	world
		.get_resource::<ProviderCachePolicy<S::Catalog>>()
		.map_or(CachePolicy::default(), |config| config.policy)
}
