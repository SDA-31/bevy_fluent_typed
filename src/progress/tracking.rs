//! Loading identities and mutation stamps for cheap publication guards.
use std::{collections::BTreeSet, ops::Deref, sync::Arc};

#[derive(Default)]
pub(crate) struct StoreVersion {
	pub(crate) identity: Option<Arc<()>>,
	pub(crate) revision: u64,
}

impl StoreVersion {
	pub(crate) fn enabled(&self) -> bool {
		self.identity.is_some()
	}

	pub(crate) fn enable(&mut self) {
		if self.identity.is_none() {
			self.identity = Some(Arc::new(()));
		}
	}

	pub(crate) fn changed(&mut self) {
		if self.enabled() {
			self.revision = self.revision.wrapping_add(1);
		}
	}
}

#[derive(Default)]
pub(crate) struct ObservedSet {
	paths: BTreeSet<&'static str>,
	pub(crate) revision: u64,
	enabled: bool,
}

impl ObservedSet {
	pub(crate) fn new() -> Self {
		Self::default()
	}

	pub(crate) fn enabled(&self) -> bool {
		self.enabled
	}

	pub(crate) fn enable(&mut self) {
		self.enabled = true;
	}

	fn changed(&mut self) {
		if self.enabled {
			self.revision = self.revision.wrapping_add(1);
		}
	}

	pub(crate) fn insert(&mut self, path: &'static str) -> bool {
		let changed = self.paths.insert(path);

		if changed {
			self.changed();
		}

		changed
	}

	pub(crate) fn remove(&mut self, path: &str) -> bool {
		let changed = self.paths.remove(path);

		if changed {
			self.changed();
		}

		changed
	}
}

impl Extend<&'static str> for ObservedSet {
	fn extend<T: IntoIterator<Item = &'static str>>(&mut self, paths: T) {
		for path in paths {
			self.insert(path);
		}
	}
}

impl Deref for ObservedSet {
	type Target = BTreeSet<&'static str>;

	fn deref(&self) -> &Self::Target {
		&self.paths
	}
}
