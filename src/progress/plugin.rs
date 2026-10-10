//! Ordinary Bevy opt-in for a typed progress subtree.
use crate::FluentScope;
use crate::bevy::prelude::*;
use std::marker::PhantomData;

/// Observe a scope and all its descendants without requesting translations.
///
/// Add through `App::add_plugins`. The owning localization plugin supplies the
/// Auto/Full/Lazy mode; either plugin order is supported. Multiple scopes of one
/// provider share a tracker, and repeated registration of a scope is idempotent.
/// Register during App setup, before `App::finish` or `App::cleanup`.
/// The selected scope and every registered descendant get their own
/// `LocalizationProgress<Scope>` resource before Startup once the base plugin is
/// installed. Selecting the root observes all registered scopes.
///
/// A root counts current demand. A group or leaf counts its fixed unique schema
/// paths, including unrequested files. Preparation readiness remains provider-wide.
pub struct LocalizationProgressPlugin<S: FluentScope>(PhantomData<fn() -> S>);

impl<S: FluentScope> LocalizationProgressPlugin<S> {
	/// Observe `S` and its registered descendants, without additional options.
	pub const fn new() -> Self {
		Self(PhantomData)
	}
}

impl<S: FluentScope> Default for LocalizationProgressPlugin<S> {
	fn default() -> Self {
		Self::new()
	}
}

impl<S: FluentScope> Plugin for LocalizationProgressPlugin<S> {
	fn build(&self, app: &mut App) {
		super::registration::request::<S>(app);
	}

	fn is_unique(&self) -> bool {
		false
	}
}
