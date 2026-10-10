//! Deferred typed messages and catalog reload notifications.
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
mod template;

use crate::bevy::{ecs as bevy_ecs, prelude::Component};
use crate::{FluentCatalog, FluentScope};
use std::{fmt, marker::PhantomData, sync::Arc};

/// Cloneable deferred formatting for a leaf, group or complete catalog.
///
/// Capture owned argument values, not an already translated string. This allows
/// stored notices to follow later language switches and hot reloads. Creating a
/// message does not load its scope; render it only after that scope is available.
/// For Decimal displays, capture the raw value and reusable per-locale formatters,
/// then select the formatter using the catalog passed to the closure. A captured
/// preformatted number would keep the old language's digits and plural category.
/// Use dedicated [ICU4X](https://docs.rs/icu/) or
/// [ICU](https://unicode-org.github.io/icu/userguide/format_parse/) formatters;
/// pass their output as ordinary String arguments. The
/// [ICU resource example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/icu)
/// shares application-owned formatters through a resource and `Arc`.
/// Changing another resource does not invalidate captured values automatically;
/// replace the binding when its value or formatter settings change.
pub struct Message<C: FluentScope>(Arc<dyn Fn(&C) -> String + Send + Sync>);

impl<C: FluentScope> Clone for Message<C> {
	fn clone(&self) -> Self {
		Self(Arc::clone(&self.0))
	}
}

impl<C: FluentScope> Message<C> {
	/// Store a thread-safe formatting closure; cloning shares the closure via `Arc`.
	pub fn new(format: impl Fn(&C) -> String + Send + Sync + 'static) -> Self {
		Self(Arc::new(format))
	}

	/// Store fixed text returned unchanged for every locale and catalog snapshot.
	pub fn literal(value: &'static str) -> Self {
		Self::new(move |_| value.into())
	}

	/// Invoke the stored formatter with this catalog; results are not cached.
	pub fn render(&self, catalog: &C) -> String {
		(self.0)(catalog)
	}
}

impl<C: FluentScope> fmt::Debug for Message<C> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("Message(<typed formatter>)")
	}
}

#[derive(Component)]
#[component(on_add = crate::bindings::added::<C>, on_remove = crate::bindings::removed::<C>)]
/// Bind an existing Bevy `Text` or `Text2d` to a leaf, group or root message.
///
/// The plugin changes text in place when the catalog or binding changes. It does
/// not spawn/despawn the entity. In default Auto, inserting the component requests
/// and retains its scope until removed; the last consumer releases demand.
/// Explicit Lazy requires manual requests. An unavailable scope clears
/// bound text until that scope becomes ready. Prefer the smallest scope the
/// message needs, so unrelated modules do not delay it.
/// Bound text contents are replaced, so keep editable drafts separate; this
/// binding does not manage or preserve text-editor state.
/// Cloning shares the formatter and its captured arguments, without capturing
/// a catalog snapshot. Creating or cloning an unattached binding is passive.
/// Each inserted clone owns demand in Auto and refreshes independently.
/// On Bevy 0.19/0.20, native BSN accepts `LocalizedText::<Scope>::new(...)`
/// and `LocalizedText::<Scope>::from(...)` directly through `FromTemplate`.
/// Number formatting belongs to the closure (see [`Message`]); shaping, visual
/// bidi ordering and font coverage belong to the renderer, not this component.
pub struct LocalizedText<C: FluentScope>(pub(crate) Message<C>);

impl<C: FluentScope> Clone for LocalizedText<C> {
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<C: FluentScope> LocalizedText<C> {
	/// Construct a binding; replace the component when captured arguments change.
	pub fn new(format: impl Fn(&C) -> String + Send + Sync + 'static) -> Self {
		Self(Message::new(format))
	}
}

impl<C: FluentScope> From<Message<C>> for LocalizedText<C> {
	fn from(message: Message<C>) -> Self {
		Self(message)
	}
}

/// The host application decides how to present successful/rejected reloads.
#[cfg_attr(feature = "bevy-0-16", derive(crate::bevy::prelude::Event))]
#[cfg_attr(not(feature = "bevy-0-16"), derive(crate::bevy::prelude::Message))]
pub enum CatalogUpdate<C: FluentCatalog> {
	/// One requested module of the selected locale passed loading and validation.
	/// Every successful reload publishes a fresh candidate; idle frames keep identity.
	Loaded {
		/// Language whose candidate was accepted, including unchanged loads.
		locale: C::Locale,
		/// Logical module whose checked candidate was published.
		path: String,
	},
	/// A load failed; a previous good value for this same-language leaf is retained.
	Rejected {
		/// Affected selected language; optional for application/provider diagnostics.
		locale: Option<C::Locale>,
		/// Logical module path; source errors may also contain the concrete asset address.
		path: String,
		/// Human-readable validation or loading diagnostic, for host-side presentation.
		error: String,
	},
}

/// Bevy system parameter for reading this provider's reload notifications.
///
/// An alias for `EventReader<CatalogUpdate<C>>` on Bevy 0.16 and
/// `MessageReader<CatalogUpdate<C>>` on newer backends. Iterate with `.read()`;
/// the underlying engine type remains usable directly.
pub type CatalogUpdateReader<'w, 's, C> =
	crate::compatibility::MessageReader<'w, 's, CatalogUpdate<C>>;

/// Retry all currently requested modules without exposing asset handles.
/// Also retries requested target leaves during locale preparation.
///
/// Per-module requests wait for the current localization-owned load, then coalesce
/// into one fresh read. Works without watching, including recovery
/// after a module was absent during the initial load.
/// Custom asset sources can send this after installing a compatible translation
/// pack. Finish the installation first and keep one coherent source revision
/// available until loading completes; this request does not snapshot an archive.
/// Automatic watching of an archive requires support from its asset source.
/// Watcher/direct AssetServer reloads bypass this queue. Bevy exposes no request
/// generation before opening its reader or in `AssetLoadFailedEvent` (including
/// `read_to_end` failures), so overlapping external reloads cannot be guaranteed
/// to publish in original request order.
#[cfg_attr(feature = "bevy-0-16", derive(crate::bevy::prelude::Event))]
#[cfg_attr(not(feature = "bevy-0-16"), derive(crate::bevy::prelude::Message))]
pub struct ReloadCatalogs<C: FluentCatalog>(PhantomData<fn() -> C>);

impl<C: FluentCatalog> Default for ReloadCatalogs<C> {
	fn default() -> Self {
		Self(PhantomData)
	}
}
