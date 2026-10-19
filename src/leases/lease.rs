//! Independent demand owners and a thread-safe queue of released owners.
use crate::FluentScope;
use crate::bevy::{ecs as bevy_ecs, prelude::Component};
use std::{
	any::TypeId,
	collections::HashMap,
	marker::PhantomData,
	sync::{
		Arc, Mutex, Weak,
		atomic::{AtomicBool, Ordering},
	},
};

/// Independent ownership of a Manual localization scope.
///
/// Keep this token for as long as its screen needs translations. Dropping it on
/// any thread releases only this owner's demand at the next publication boundary.
/// Other leases and explicit `load` requests remain active. This token is also a
/// Bevy component: despawning its entity releases its demand.
///
/// Tokens cannot be cloned; use `Arc<ModuleLease<S>>` for shared ownership of one
/// token, or call `Localization::hold` again for an independent owner.
/// `Localization::hold` follows the active language; a lease acquired through
/// `Localization::for_locale_mut` retains that specific language. A token
/// does not keep the controller or its catalogs alive after the controller drops.
#[derive(Component, Debug)]
#[must_use = "keep the lease while its scope is needed; dropping it releases demand"]
pub struct ModuleLease<S: FluentScope> {
	releases: Weak<ReleaseQueue>,
	marker: PhantomData<fn() -> S>,
}

impl<S: FluentScope> Drop for ModuleLease<S> {
	fn drop(&mut self) {
		let Some(releases) = self.releases.upgrade() else {
			return;
		};

		let mut dropped = releases
			.dropped
			.lock()
			.unwrap_or_else(|error| error.into_inner());
		dropped.push(TypeId::of::<S>());
		// Set under the lock so a concurrent drain cannot lose the notification.
		releases.pending.store(true, Ordering::Release);
	}
}

/// Dropped lease notifications accepted from any thread.
/// Writers set pending while holding dropped's mutex so draining cannot lose a notification.
#[derive(Debug, Default)]
struct ReleaseQueue {
	pending: AtomicBool,
	dropped: Mutex<Vec<TypeId>>,
}

/// Controller-local owner count and leaf paths for one leased scope.
struct Demand {
	paths: &'static [&'static str],
	owners: usize,
}

/// Controller-owned lease counts and release queue; tokens hold only weak queue links.
#[derive(Default)]
pub(crate) struct LeaseRequests {
	demands: HashMap<TypeId, Demand>,
	releases: Arc<ReleaseQueue>,
}

impl LeaseRequests {
	pub(crate) fn hold<S: FluentScope>(&mut self) -> ModuleLease<S> {
		let demand = self.demands.entry(TypeId::of::<S>()).or_insert(Demand {
			paths: S::module_paths(),
			owners: 0,
		});
		demand.owners += 1;

		ModuleLease {
			releases: Arc::downgrade(&self.releases),
			marker: PhantomData,
		}
	}

	pub(crate) fn paths(&self) -> impl Iterator<Item = &'static [&'static str]> + '_ {
		self.demands.values().map(|demand| demand.paths)
	}

	pub(crate) fn has_releases(&self) -> bool {
		self.releases.pending.load(Ordering::Acquire)
	}

	pub(crate) fn release_dropped(&mut self) -> bool {
		if !self.releases.pending.swap(false, Ordering::Acquire) {
			return false;
		}

		let mut dropped = self
			.releases
			.dropped
			.lock()
			.unwrap_or_else(|error| error.into_inner());
		let mut changed = false;

		for id in dropped.drain(..) {
			let demand = self.demands.get_mut(&id).expect("live lease demand");
			demand.owners -= 1;

			if demand.owners == 0 {
				self.demands.remove(&id);
				changed = true;
			}
		}

		changed
	}
}
