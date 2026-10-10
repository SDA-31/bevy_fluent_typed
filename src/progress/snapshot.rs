//! Loading counts without collecting per-module details or requesting data.
use crate::{FluentCatalog, ModuleStatus, ModuleStore};

/// Latest attempts and usable snapshots across unique logical module paths.
///
/// `ready + loading + failed + unloaded == total`. Availability is independent:
/// a loading or failed same-locale reload may retain a usable last-good snapshot.
/// Counts describe modules, not byte or elapsed-time progress. Per-scope queries
/// include unrequested leaves; the native resource counts current physical demand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadingProgress<L> {
	/// Compiled locale represented by this snapshot.
	pub locale: L,
	/// Number of unique logical modules being inspected.
	pub total: usize,
	/// Modules whose latest attempt succeeded.
	pub ready: usize,
	/// Modules whose latest attempt is loading.
	pub loading: usize,
	/// Modules whose latest attempt failed.
	pub failed: usize,
	/// Modules without a recorded attempt for this locale.
	pub unloaded: usize,
	/// Modules with a usable same-locale snapshot, including last-good data.
	pub available: usize,
}

pub(crate) fn inspect<C: FluentCatalog>(
	store: &ModuleStore<C>,
	paths: impl ExactSizeIterator<Item = &'static str>,
) -> LoadingProgress<C::Locale> {
	let mut progress = LoadingProgress {
		locale: store.locale,
		total: paths.len(),
		ready: 0,
		loading: 0,
		failed: 0,
		unloaded: 0,
		available: 0,
	};

	for path in paths {
		let usable = store
			.leaves
			.get(path)
			.is_some_and(|id| store.values.contains_key(id));

		match store.states.get(path) {
			Some(ModuleStatus::Ready) => progress.ready += 1,
			Some(ModuleStatus::Loading) => progress.loading += 1,
			Some(ModuleStatus::Failed(_)) => progress.failed += 1,
			Some(ModuleStatus::Unloaded) | None => progress.unloaded += 1,
		}

		progress.available += usize::from(usable);
	}

	progress
}
