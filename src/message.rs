//! Deferred typed messages and catalog reload notifications.
use crate::FluentCatalog;
use crate::bevy::{ecs as bevy_ecs, prelude::Component};
use std::{fmt, marker::PhantomData, sync::Arc};

/// Cloneable deferred typed formatting, evaluated against the current catalog.
///
/// Capture owned argument values, not an already translated string. This allows
/// stored notices to follow later language switches and hot reloads.
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

	/// Store fixed text returned unchanged for every locale and catalog snapshot.
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
/// not spawn/despawn the entity. Bound text contents are replaced, so keep editable
/// drafts separate; this binding does not manage or preserve text-editor state.
/// Number formatting belongs to the closure (see [`Message`]); shaping, visual
/// bidi ordering and font coverage belong to the renderer, not this component.
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
#[cfg_attr(feature = "bevy-0-16", derive(crate::bevy::prelude::Event))]
#[cfg_attr(not(feature = "bevy-0-16"), derive(crate::bevy::prelude::Message))]
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

/// Bevy system parameter for reading this provider's reload notifications.
///
/// An alias for `EventReader<CatalogUpdate<C>>` on Bevy 0.16 and
/// `MessageReader<CatalogUpdate<C>>` on newer backends. Iterate with `.read()`;
/// the underlying engine type remains usable directly.
pub type CatalogUpdateReader<'w, 's, C> =
	crate::compatibility::MessageReader<'w, 's, CatalogUpdate<C>>;

/// Request one aggregate reload without exposing asset handles to application code.
///
/// Requests processed together coalesce into one asynchronous reload of the
/// definition and declared modules. Works without watching, including recovery
/// after a module was absent during the initial load.
/// Custom asset sources can send this after installing a compatible translation
/// pack. Finish the installation first and keep one coherent source revision
/// available until loading completes; this request does not snapshot an archive.
/// Automatic watching of an archive requires support from its asset source.
#[cfg_attr(feature = "bevy-0-16", derive(crate::bevy::prelude::Event))]
#[cfg_attr(not(feature = "bevy-0-16"), derive(crate::bevy::prelude::Message))]
pub struct ReloadCatalogs<C: FluentCatalog>(PhantomData<fn() -> C>);

impl<C: FluentCatalog> Default for ReloadCatalogs<C> {
	fn default() -> Self {
		Self(PhantomData)
	}
}
