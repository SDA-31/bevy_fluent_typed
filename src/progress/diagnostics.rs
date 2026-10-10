//! Detailed snapshots collected only by explicit diagnostics queries.
use crate::{FluentCatalog, ModuleStatus, ModuleStore};

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
) -> Vec<ModuleDiagnostic> {
	paths
		.map(|path| ModuleDiagnostic {
			path,
			status: store
				.states
				.get(path)
				.cloned()
				.unwrap_or(ModuleStatus::Unloaded),
			usable: store
				.leaves
				.get(path)
				.is_some_and(|id| store.values.contains_key(id)),
		})
		.collect()
}
