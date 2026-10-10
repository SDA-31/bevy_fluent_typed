//! Counts and latest-attempt diagnostics without requesting catalog data.
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
	/// Latest attempt and availability for each unique module, sorted by path.
	pub modules: Vec<ModuleDiagnostic>,
}

/// Latest attempt and snapshot availability for one logical module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDiagnostic {
	/// Logical FTL path, independent of the byte source or transport.
	pub path: &'static str,
	/// Latest attempt status, retaining original failure details.
	pub status: ModuleStatus,
	/// Whether a same-locale snapshot is still available.
	pub usable: bool,
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
		modules: Vec::with_capacity(paths.len()),
	};

	for path in paths {
		let status = store
			.states
			.get(path)
			.cloned()
			.unwrap_or(ModuleStatus::Unloaded);
		let usable = store
			.leaves
			.get(path)
			.is_some_and(|id| store.values.contains_key(id));

		match &status {
			ModuleStatus::Ready => progress.ready += 1,
			ModuleStatus::Loading => progress.loading += 1,
			ModuleStatus::Failed(_) => progress.failed += 1,
			ModuleStatus::Unloaded => progress.unloaded += 1,
		}

		progress.available += usize::from(usable);
		progress.modules.push(ModuleDiagnostic {
			path,
			status,
			usable,
		});
	}

	progress
}
