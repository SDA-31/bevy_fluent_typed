use crate::bevy::{ecs as bevy_ecs, prelude::Component};
use crate::message::MsgIdentity;
use crate::{FluentScope, LocalizationTarget, Msg};
use std::marker::PhantomData;

/// Per-binding metadata lives beside every target, regardless of its type.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub(super) struct State<S: FluentScope, T: LocalizationTarget> {
	pub(super) message: Option<MsgIdentity<S>>,
	#[allow(
		clippy::box_collection,
		reason = "Keep uncached per-binding metadata compact; allocate only for Rendered"
	)]
	pub(super) text: Option<Box<String>>,
	pub(super) pass: u64,
	observed: u32,
	confirmed: bool,
	pub(super) pending: bool,
	marker: PhantomData<fn() -> T>,
}

impl<S: FluentScope, T: LocalizationTarget> Default for State<S, T> {
	fn default() -> Self {
		Self {
			message: None,
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
