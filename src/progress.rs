//! On-demand inspection of active-language attempts and retained leaf snapshots.
use crate::ModuleStatus;

/// Loading attempts and usable snapshots across a scope's unique logical leaves.
///
/// Counts describe the active locale when queried, including unrequested leaves.
/// A privately prepared target is excluded; inspect its preparation status separately.
/// `ready + loading + failed + unloaded == total`. Availability is independent:
/// a loading or failed reload can still have a usable last-good snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadingProgress<L> {
	/// Active compiled locale at the time of inspection.
	pub locale: L,
	/// Number of unique logical leaves required by the scope.
	pub total: usize,
	/// Leaves whose latest attempt succeeded.
	pub ready: usize,
	/// Leaves currently loading or retrying.
	pub loading: usize,
	/// Leaves whose latest attempt failed.
	pub failed: usize,
	/// Leaves with no recorded attempt for the active locale.
	pub unloaded: usize,
	/// Leaves with a usable active-locale snapshot, including last-good values.
	pub available: usize,
	/// One diagnostic per unique leaf, sorted by logical path.
	pub modules: Vec<ModuleDiagnostic>,
}

/// Current attempt and snapshot availability for one logical leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDiagnostic {
	/// Logical FTL path, independent of the byte source or transport.
	pub path: &'static str,
	/// Latest attempt status; failed attempts retain their original error details.
	pub status: ModuleStatus,
	/// Whether an active-locale snapshot can still be borrowed from the store.
	pub usable: bool,
}
