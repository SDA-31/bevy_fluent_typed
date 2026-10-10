//! Diagnostics-only identities and mutation stamps for cheap publication guards.
use std::{collections::BTreeSet, ops::Deref, sync::Arc};

#[derive(Default)]
pub(crate) struct StoreVersion {
	pub(crate) identity: Arc<()>,
	pub(crate) revision: u64,
}

impl StoreVersion {
	pub(crate) fn changed(&mut self) {
		self.revision = self.revision.wrapping_add(1);
	}
}

#[derive(Default)]
pub(crate) struct ObservedSet {
	paths: BTreeSet<&'static str>,
	pub(crate) revision: u64,
}

impl ObservedSet {
	pub(crate) fn new() -> Self {
		Self::default()
	}

	pub(crate) fn insert(&mut self, path: &'static str) -> bool {
		let changed = self.paths.insert(path);
		self.revision = self.revision.wrapping_add(u64::from(changed));
		changed
	}

	pub(crate) fn remove(&mut self, path: &str) -> bool {
		let changed = self.paths.remove(path);
		self.revision = self.revision.wrapping_add(u64::from(changed));
		changed
	}

	pub(crate) fn clear(&mut self) {
		if !self.paths.is_empty() {
			self.paths.clear();
			self.revision = self.revision.wrapping_add(1);
		}
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
