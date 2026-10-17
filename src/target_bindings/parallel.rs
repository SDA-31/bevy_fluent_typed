use crate::bevy::tasks::ComputeTaskPool;

// Counts, not frame budgets. Keep ordinary UI on the calling thread.
// Minimax calibration on 2/4/6 workers: equivalent primary optima 513..8192;
// 4096 minimizes the mean ratio among those optima on the calibration grid.
// That calibration used the HashMap pipeline; it does not prove an optimum
// for the native fused query, which currently retains the same cutoffs.
pub(super) const MIN_FORMATTING_BINDINGS: usize = 4 << 10;
pub(super) const MIN_REPAIR_BINDINGS: usize = 32 << 10;

pub(super) fn should_parallelize(count: usize, minimum: usize) -> bool {
	count >= minimum && ComputeTaskPool::try_get().is_some_and(|pool| pool.thread_num() > 1)
}
