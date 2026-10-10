//! Ordinary Bevy opt-in for one typed progress view.
use crate::FluentScope;
use crate::bevy::prelude::*;
use std::marker::PhantomData;

/// Observe a root, group or leaf without requesting its translations.
///
/// Add through `App::add_plugins`. The owning localization plugin supplies the
/// Full/Lazy mode; either plugin order is supported. Multiple scopes of one
/// provider share a tracker, and repeated registration of a scope is idempotent.
/// Register during App setup, before `App::finish` or `App::cleanup`.
/// Each selected scope gets its own `LocalizationProgress<S>` resource before
/// Startup once the base plugin is installed.
///
/// A root counts current demand. A group or leaf counts its fixed unique schema
/// paths, including unrequested files. Preparation readiness remains provider-wide.
pub struct LocalizationProgressPlugin<S: FluentScope>(PhantomData<fn() -> S>);

impl<S: FluentScope> Default for LocalizationProgressPlugin<S> {
	fn default() -> Self {
		Self(PhantomData)
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
