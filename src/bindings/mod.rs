//! Entity-owned refresh jobs shared by native and application-defined targets.
mod registry;

#[cfg(all(
	test,
	feature = "manifest",
	any(feature = "bevy-0-19", feature = "bevy-0-20")
))]
pub(crate) use registry::dispatch;
pub(crate) use registry::{Binding, added, register, removed};
