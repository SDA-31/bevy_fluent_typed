//! Deferred typed messages and game-independent reload notifications.
use crate::FluentCatalog;
use bevy::prelude::Component;
use std::{fmt, marker::PhantomData, sync::Arc};

/// Cloneable deferred typed formatting, evaluated against the current catalog.
///
/// Capture owned argument values, not an already translated string. This allows
/// stored notices to follow later language switches and hot reloads.
pub struct Message<C: FluentCatalog>(Arc<dyn Fn(&C) -> String + Send + Sync>);

impl<C: FluentCatalog> Clone for Message<C> {
	fn clone(&self) -> Self {
		Self(Arc::clone(&self.0))
	}
}

impl<C: FluentCatalog> Message<C> {
	/// Store a thread-safe formatting closure; cloning shares the closure via `Arc`.
	pub fn new(format: impl Fn(&C) -> String + Send + Sync + 'static) -> Self {
		Self(Arc::new(format))
	}

	/// Only invariant punctuation/empty content belongs outside a catalog.
	pub fn literal(value: &'static str) -> Self {
		Self::new(move |_| value.into())
	}

	/// Invoke the stored formatter with this catalog; results are not cached.
	pub fn render(&self, catalog: &C) -> String {
		(self.0)(catalog)
	}
}

impl<C: FluentCatalog> fmt::Debug for Message<C> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("Message(<typed formatter>)")
	}
}

#[derive(Component)]
/// Bind an existing Bevy `Text` or `Text2d` to a deferred message.
///
/// The plugin changes text in place when the catalog or binding changes. It does
/// not spawn/despawn the entity or reset text-editor state.
pub struct LocalizedText<C: FluentCatalog>(pub(crate) Message<C>);

impl<C: FluentCatalog> LocalizedText<C> {
	/// Construct a binding; replace the component when captured arguments change.
	pub fn new(format: impl Fn(&C) -> String + Send + Sync + 'static) -> Self {
		Self(Message::new(format))
	}
}

impl<C: FluentCatalog> From<Message<C>> for LocalizedText<C> {
	fn from(message: Message<C>) -> Self {
		Self(message)
	}
}

/// The host application decides how to present successful/rejected reloads.
#[derive(bevy::prelude::Message)]
pub enum CatalogUpdate<C: FluentCatalog> {
	/// A complete language passed loading and validation; it need not be active.
	/// Unchanged sources retain the existing snapshot instead of replacing it.
	Loaded {
		/// Language whose candidate was accepted, including unchanged loads.
		locale: C::Locale,
	},
	/// A load failed; the previous catalog remains usable.
	Rejected {
		/// Affected language, or `None` for aggregate definition/address failures.
		locale: Option<C::Locale>,
		/// Bevy path of the definition asset; the diagnostic identifies failing modules.
		path: String,
		/// Human-readable validation or loading diagnostic, for host-side presentation.
		error: String,
	},
}

/// Request one aggregate reload without exposing asset handles to gameplay.
///
/// Requests processed together coalesce into one asynchronous reload of the
/// definition and declared modules. Works without watching, including recovery
/// after a module was absent during the initial load.
#[derive(bevy::prelude::Message)]
pub struct ReloadCatalogs<C: FluentCatalog>(PhantomData<fn() -> C>);

impl<C: FluentCatalog> Default for ReloadCatalogs<C> {
	fn default() -> Self {
		Self(PhantomData)
	}
}
