//! Deferred typed messages and catalog reload notifications.
use crate::bevy::ecs as bevy_ecs;
use crate::{FluentCatalog, FluentScope};
use std::{
	fmt,
	marker::PhantomData,
	ptr,
	sync::{Arc, Weak},
};

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
/// Automatic bindings may invoke formatters concurrently on Bevy's compute
/// pool. Do not rely on entity iteration order or invocation on the main thread.
pub struct Msg<C: FluentScope>(Arc<dyn Fn(&C) -> String + Send + Sync>);

/// Deprecated name for [`Msg`]; constructors and binding conversions are unchanged.
#[deprecated(note = "use `Msg` instead")]
pub type Message<C> = Msg<C>;

/// Weak formatter identity used to invalidate bindings without retaining captures.
/// Its control block prevents allocation-address reuse until the identity retires.
pub(crate) struct MsgIdentity<C: FluentScope>(Weak<dyn Fn(&C) -> String + Send + Sync>);

impl<C: FluentScope> MsgIdentity<C> {
	pub(crate) fn matches(&self, message: &Msg<C>) -> bool {
		// Keep the allocation's identity alive without retaining captured values.
		// The weak control block prevents address reuse until this identity retires.
		ptr::addr_eq(self.0.as_ptr(), Arc::as_ptr(&message.0))
	}
}

impl<C: FluentScope> Clone for Msg<C> {
	fn clone(&self) -> Self {
		Self(Arc::clone(&self.0))
	}
}

impl<C: FluentScope> Msg<C> {
	/// Store a thread-safe formatting closure; cloning shares the closure via `Arc`.
	pub fn new(format: impl Fn(&C) -> String + Send + Sync + 'static) -> Self {
		Self(Arc::new(format))
	}

	#[cfg(test)]
	pub(crate) fn same_formatter(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.0, &other.0)
	}

	pub(crate) fn identity(&self) -> MsgIdentity<C> {
		MsgIdentity(Arc::downgrade(&self.0))
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

impl<C: FluentScope> fmt::Debug for Msg<C> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("Msg(<typed formatter>)")
	}
}

/// The host application decides how to present successful/rejected acquisitions.
/// Automatic switching failures are reported before switching; successful target
/// leaves report `Loaded` when their locale is committed. Manual preparation
/// failures remain available through `Localization::preparation_status`.
#[cfg_attr(feature = "bevy-0-16", derive(crate::bevy::prelude::Event))]
#[cfg_attr(not(feature = "bevy-0-16"), derive(crate::bevy::prelude::Message))]
pub enum CatalogUpdate<C: FluentCatalog> {
	/// One requested module of the reported locale passed loading and validation.
	/// Every successful reload publishes a fresh candidate; idle frames keep identity.
	Loaded {
		/// Language whose candidate was accepted, including unchanged loads.
		locale: C::Locale,
		/// Logical module whose checked candidate was published.
		path: String,
	},
	/// A load failed. Same-language reloads retain the last good leaf; failed
	/// target preparation keeps the active locale's resources usable.
	Rejected {
		/// Affected active or prepared target language; optional for provider diagnostics.
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
