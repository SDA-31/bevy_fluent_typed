#[cfg(not(any(feature = "bevy-0-19", feature = "bevy-0-20")))]
use crate::bevy::ecs::query::QueryData as TargetData;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
use crate::bevy::ecs::query::SingleEntityQueryData as TargetData;
use crate::bevy::{
	ecs as bevy_ecs,
	ecs::query::ArchetypeFilter,
	prelude::{Component, EntityWorldMut},
};
use crate::{FluentScope, LocalizationUpdate, Msg};
use std::marker::PhantomData;

/// Choose when a target receives its translated value in PostUpdate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetUpdatePhase {
	/// Before UI/text layout.
	BeforeLayout,
	/// After UI accessibility producers and before export on Bevy 0.19/0.20.
	/// Older engines order this only after the localization text refresh.
	AfterLayout,
}

/// Whether an adapter finished through its query or needs exclusive entity access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateResult {
	/// Updated, already correct, or intentionally waiting for application data.
	Done,
	/// Run `apply` after the query borrows end, without forcing a message read.
	Deferred,
}

/// Apply translated text through one native Bevy query and optional structural work.
///
/// Implement on a local component; [`Default`] is not required. Built-in and
/// application adapters use this same contract. Non-component field markers are
/// not supported.
///
/// `Data` and `Filter` describe components on the binding's entity; the engine
/// adds `Entity`, `Localized<Self, _>` and private tracking metadata. Do not request
/// the binding itself or whole-entity mutable access in `Data`.
///
/// A minimal manual adapter declares Data/Filter and implements `apply`. The
/// default `update` requests that sequential callback without forcing a message
/// read. Override `update` for disjoint query writes; large sets may run on Bevy
/// workers. `needs_update` optionally skips Idle work and informs scheduling.
/// Neither callback may change its binding; `apply` must not despawn the entity.
///
/// Spawn the target and its binding together. When the binding's deferred add
/// callback runs, an absent target causes the binding to be removed with a warning.
/// No target or neighboring component is created, even if it implements Default.
/// After a successful attachment, required query data can wait while an
/// application-owned target is absent. Reinserting it resumes updates.
///
/// Application components implement this trait directly. Only Data/Filter and
/// `apply` are required; override the default hooks for change tracking or direct
/// query writes when needed. Applications own target and neighbor construction.
///
/// Compare before writing to preserve change ticks. Combine context invalidation
/// with `LocalizationUpdate::component_changed` for native change tracking that
/// excludes confirmed own writes. Explicit `bypass_change_detection` writes
/// remain untracked. `CachePolicy::None` formats only when a callback reads text;
/// Rendered retains output until invalidated. Deferred `matches` verification
/// occurs after the whole batch and its hooks, and is available to every adapter.
/// AccessibleLabel uses its normal immutable insertion hooks; the runtime does
/// not independently own, repair or recreate AccessibilityNode.
///
/// A component without Default can be a target:
///
/// ```
/// use bevy_fluent_typed::bevy::{ecs as bevy_ecs, prelude::{Component, EntityWorldMut}};
/// use bevy_fluent_typed::{LocalizationTarget, LocalizationUpdate};
///
/// #[derive(Component)]
/// struct NoDefault(String);
///
/// impl LocalizationTarget for NoDefault {
///     type Data = &'static mut Self;
///     type Filter = ();
///
///     fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
///         let Some(target) = entity.get_mut::<Self>() else {
///             return;
///         };
///
///         update.set_text(target.map_unchanged(|target| &mut target.0));
///     }
/// }
/// ```
///
/// A non-component field marker cannot be a target:
///
/// ```compile_fail,E0277
/// use bevy_fluent_typed::bevy::prelude::EntityWorldMut;
/// use bevy_fluent_typed::{LocalizationTarget, LocalizationUpdate};
///
/// #[derive(Default)]
/// struct FieldMarker;
///
/// impl LocalizationTarget for FieldMarker {
///     type Data = ();
///     type Filter = ();
///
///     fn apply(_: &mut EntityWorldMut<'_>, _: &mut LocalizationUpdate<'_>) {}
/// }
/// ```
pub trait LocalizationTarget: Component {
	/// Components on the binding's entity, using native Bevy query syntax.
	///
	/// Reference lifetimes in this description are `'static`; actual items borrow
	/// the running query. Bevy 0.19/0.20 require `SingleEntityQueryData` so separate
	/// rows remain safe for parallel iteration. Older backends use `QueryData`.
	type Data: TargetData + 'static;

	/// Structural membership: `()`, `With`, `Without`, tuples and nested `Or`.
	///
	/// `Changed`/`Added` are not membership filters: a catalog can change without
	/// changing the target. Inspect ticks in Data instead. While excluded, the
	/// target is left untouched; reentry uses the latest catalog and Msg. Installing
	/// the binding still retains its scope in Lazy, regardless of this filter.
	///
	/// ```compile_fail
	/// use bevy_fluent_typed::bevy::{ecs as bevy_ecs, prelude::{Changed, Component, EntityWorldMut, Text}};
	/// use bevy_fluent_typed::LocalizationTarget;
	///
	/// #[derive(Component, Default)]
	/// struct ChangedOnly;
	/// impl LocalizationTarget for ChangedOnly {
	///     type Data = Option<&'static mut Text>;
	///     type Filter = Changed<Text>;
	///
	///     fn apply(_: &mut EntityWorldMut<'_>, _: &mut bevy_fluent_typed::LocalizationUpdate<'_>) {}
	/// }
	/// ```
	type Filter: ArchetypeFilter + 'static;

	/// Text layout is the default; semantic accessibility labels run after layout.
	const PHASE: TargetUpdatePhase = TargetUpdatePhase::BeforeLayout;

	/// Cheap optional eligibility check for work scheduling and Idle skipping.
	///
	/// The default always runs `update`. This method only inspects query data;
	/// it must have no side effects and may run on a compute worker. Scheduling
	/// and execution can evaluate it more than once in one pass. Returning false
	/// postpones work: catalog/readiness invalidation stays pending until resumed.
	fn needs_update(_data: &TargetItem<'_, '_, Self>, _update: &LocalizationUpdate<'_>) -> bool {
		true
	}

	/// Handle existing query data without structural changes.
	///
	/// Done includes unchanged output and an intentional wait for application data.
	/// Deferred requests `apply` after query borrows end. The default defers; no text
	/// is requested until a callback calls `LocalizationUpdate::text`. This method
	/// may run on Bevy workers; do not rely on thread identity or entity order.
	fn update(
		_data: TargetItem<'_, '_, Self>,
		_update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		UpdateResult::Deferred
	}

	/// Finish an update that [`Self::update`] deferred.
	///
	/// Runs sequentially after the query pass. Read `update.text()` only if needed;
	/// already available neighbor data can be used without invoking Msg. If query
	/// membership changes before this callback runs, its write remains pending.
	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>);

	/// Confirm a deferred write after the whole batch and its insertion hooks.
	///
	/// Return true only when the relevant outputs equal `text`. A successful
	/// confirmation prevents own writes from looking like application edits on
	/// the next pass. The default does not acknowledge them; it remains correct
	/// but can require an additional message read with output caching disabled.
	fn matches(_data: TargetReadItem<'_, '_, Self>, _text: &str) -> bool {
		false
	}
}

/// One item of an adapter's native query, with the query's actual borrow lifetimes.
///
/// This alias keeps adapter signatures consistent across the supported Bevy
/// versions; Bevy 0.16 has only one lifetime on its native `QueryItem` alias.
#[cfg(feature = "bevy-0-16")]
pub type TargetItem<'w, 's, T> =
	crate::bevy::ecs::query::QueryItem<'w, <T as LocalizationTarget>::Data>;

/// One item of an adapter's native query, with the query's actual borrow lifetimes.
#[cfg(not(feature = "bevy-0-16"))]
pub type TargetItem<'w, 's, T> =
	crate::bevy::ecs::query::QueryItem<'w, 's, <T as LocalizationTarget>::Data>;

/// Read-only view of an adapter's same query, used for deferred confirmation.
#[cfg(feature = "bevy-0-16")]
pub type TargetReadItem<'w, 's, T> = crate::bevy::ecs::query::QueryItem<
	'w,
	<<T as LocalizationTarget>::Data as crate::bevy::ecs::query::QueryData>::ReadOnly,
>;

/// Read-only view of an adapter's same query, used for deferred confirmation.
#[cfg(not(feature = "bevy-0-16"))]
pub type TargetReadItem<'w, 's, T> = crate::bevy::ecs::query::QueryItem<
	'w,
	's,
	<<T as LocalizationTarget>::Data as crate::bevy::ecs::query::QueryData>::ReadOnly,
>;

#[derive(Component)]
#[component(on_add = crate::bindings::added::<Localized<T, S>>, on_remove = crate::bindings::removed::<Localized<T, S>>)]
/// Bind a deferred message to a Bevy text component or an application-defined target.
///
/// Target adapters implement [trait@LocalizationTarget]. No registration or user
/// refresh system is needed. In Lazy, insertion retains the message's scope until
/// the last consumer disappears; creating/cloning an unattached binding is passive.
/// Full and Manual retain their loading policy. An unavailable catalog supplies
/// an empty string; locale preparation keeps the active translation until commit.
/// By default, formatting runs when the catalog, message or eligibility changes.
/// The provider's [`crate::CachePolicy`] can disable output retention; native
/// target repair then formats again, and custom adapters request text lazily.
/// It follows the provider's active language by default; [`Self::with_locale`]
/// selects a fixed language. For message reuse or later binding replacements,
/// consider keeping a separate [`Msg`] and constructing bindings where they are
/// attached. A retained message can be reused when only the selected language
/// changes; new captured arguments require a new message.
/// Each target should have exactly one localization writer.
/// Built-in text targets update their selected component only. Custom adapters
/// can synchronize neighbors declared in Data. AccessibleLabel uses Bevy hooks;
/// the runtime does not independently manage its AccessibilityNode.
/// The target comes first: `Localized::<Text, Scope>` or
/// `Localized::<Text, _>::from(message)` when the scope can be inferred.
/// Spawn the target alongside this binding; Default is not required and no
/// component is created automatically. If the target is missing when the deferred
/// add callback runs, the binding is removed with a warning naming the entity,
/// target type and scope. Later target removal is respected, including across
/// catalog or Msg changes. Reinsert the target to resume updates. Removing and
/// adding the binding checks target presence again. Applications own neighboring
/// components. Non-component field markers are not supported.
/// A `TextSpan` binding requires an application-owned parent text hierarchy; it
/// never creates a root `Text`. Each clone owns its own demand and refreshes
/// independently, sharing the formatter and owned arguments through `Arc`.
/// On Bevy 0.19/0.20, native BSN accepts the typed `new`/`from` constructors.
/// Bound content is replaced; keep editable drafts in application-owned targets.
/// Number formatting belongs to the closure (see [`crate::Msg`]); shaping, visual
/// bidi ordering and font coverage remain the renderer's responsibility.
pub struct Localized<T: LocalizationTarget, S: FluentScope> {
	pub(crate) message: Msg<S>,
	pub(crate) locale: Option<<S::Catalog as crate::FluentCatalog>::Locale>,
	marker: PhantomData<fn() -> T>,
}

impl<T: LocalizationTarget, S: FluentScope> Localized<T, S> {
	/// Pin this binding to a compiled language while retaining automatic updates.
	pub fn with_locale(mut self, locale: <S::Catalog as crate::FluentCatalog>::Locale) -> Self {
		assert!(
			<S::Catalog as crate::FluentCatalog>::locales().contains(&locale),
			"locale must belong to the provider"
		);
		self.locale = Some(locale);
		self
	}

	/// Capture a formatter without reading or requesting the scope.
	pub fn new(format: impl Fn(&S) -> String + Send + Sync + 'static) -> Self {
		Self::from(Msg::new(format))
	}
}

impl<T: LocalizationTarget, S: FluentScope> From<Msg<S>> for Localized<T, S> {
	fn from(message: Msg<S>) -> Self {
		Self {
			message,
			locale: None,
			marker: PhantomData,
		}
	}
}

impl<T: LocalizationTarget, S: FluentScope> Clone for Localized<T, S> {
	fn clone(&self) -> Self {
		Self {
			message: self.message.clone(),
			locale: self.locale,
			marker: PhantomData,
		}
	}
}
