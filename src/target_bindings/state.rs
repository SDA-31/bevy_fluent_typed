use crate::bevy::{ecs as bevy_ecs, prelude::Component};
use crate::message::MsgIdentity;
use crate::{FluentScope, LocalizationTarget, Msg};
use std::marker::PhantomData;

/// Invalidation and observation state owned by one entity's (scope, target) binding.
///
/// Query-pass counters track eligibility reentry; Bevy change ticks distinguish
/// application edits from confirmed adapter writes. Output retention is optional,
/// but identity and unread invalidations survive with either cache policy.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub(super) struct State<S: FluentScope, T: LocalizationTarget> {
	pub(super) message: Option<MsgIdentity<S>>,
	pub(super) locale: Option<<S::Catalog as crate::FluentCatalog>::Locale>,
	pub(super) snapshot: Option<u64>,
	/// Binding owners share one demand allocation per scope and language.
	pub(super) consumer: Option<Ownership<S>>,
	#[allow(
		clippy::box_collection,
		reason = "Keep uncached per-binding metadata compact; allocate only for Rendered"
	)]
	/// Last rendered output; absent under CachePolicy::None.
	pub(super) text: Option<Box<String>>,
	/// Last acknowledged query pass, independent of Bevy's change ticks.
	pub(super) pass: u64,
	/// Last confirmed adapter write/observation tick, valid only when confirmed.
	observed: u32,
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
			observed: 0,
			confirmed: false,
			pending: true,
			marker: PhantomData,
		}
	}
}

impl<S: FluentScope, T: LocalizationTarget> State<S, T> {
	pub(super) fn observed(&self) -> Option<u32> {
		self.confirmed.then_some(self.observed)
	}

	pub(super) fn confirm(&mut self, tick: u32) {
		self.observed = tick;
		self.confirmed = true;
	}

	pub(super) fn current(&self, message: &Msg<S>, pass: u64) -> bool {
		self.pass == pass
			&& self
				.message
				.as_ref()
				.is_some_and(|value| value.matches(message))
	}
}

/// Shared demand ownership and the language for which it was acquired.
pub(super) struct Ownership<S: FluentScope> {
	pub(super) locale: Option<<S::Catalog as crate::FluentCatalog>::Locale>,
	pub(super) _token: std::sync::Arc<crate::demand::Consumer>,
}
