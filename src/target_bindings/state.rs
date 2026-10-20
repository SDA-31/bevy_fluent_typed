use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{Component, DetectChanges, Mut},
};
use crate::message::MsgIdentity;
use crate::state::SnapshotStamp;
use crate::{FluentScope, LocalizationTarget, Msg};
use std::{marker::PhantomData, ops::Deref, sync::Arc};

/// Invalidation and observation state owned by one entity's (scope, target) binding.
///
/// Query-pass counters track eligibility reentry; Bevy change ticks distinguish
/// application edits from confirmed adapter writes. Output retention is optional,
/// but identity and unread invalidations survive with either cache policy.
/// The component's native changed tick records the last confirmed observation;
/// Bevy ages it automatically, including while this adapter is not running.
/// Other metadata mutations must bypass change detection to preserve that clock.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub(super) struct State<S: FluentScope, T: LocalizationTarget> {
	pub(super) message: Option<MsgIdentity<S>>,
	pub(super) locale: Option<<S::Catalog as crate::FluentCatalog>::Locale>,
	pub(super) snapshot: Option<SnapshotStamp>,
	/// Binding owners share one demand allocation per scope and language.
	pub(super) consumer: Option<Arc<crate::demand::Consumer>>,
	#[allow(
		clippy::box_collection,
		reason = "Keep uncached per-binding metadata compact; allocate only for Rendered"
	)]
	/// Last rendered output; absent under CachePolicy::None.
	pub(super) text: Option<Box<String>>,
	/// Last acknowledged query pass, independent of Bevy's change ticks.
	pub(super) pass: u64,
	/// Whether an observed tick has been established for this binding lifetime.
	confirmed: bool,
	/// Invalidation remains pending until an applicable callback consumes its text.
	pub(super) pending: bool,
	marker: PhantomData<fn() -> T>,
}

impl<S: FluentScope, T: LocalizationTarget> Default for State<S, T> {
	fn default() -> Self {
		Self {
			message: None,
			locale: None,
			snapshot: None,
			consumer: None,
			text: None,
			pass: 0,
			confirmed: false,
			pending: true,
			marker: PhantomData,
		}
	}
}

impl<S: FluentScope, T: LocalizationTarget> State<S, T> {
	pub(super) fn observed(state: &(impl DetectChanges + Deref<Target = Self>)) -> Option<u32> {
		state.confirmed.then_some(state.last_changed().get())
	}

	/// This normal Mut write records the caller's native query or World tick.
	pub(super) fn confirm(state: &mut Mut<'_, Self>) {
		state.confirmed = true;
	}

	pub(super) fn current(&self, message: &Msg<S>, pass: u64) -> bool {
		self.pass == pass
			&& self
				.message
				.as_ref()
				.is_some_and(|value| value.matches(message))
	}
}
