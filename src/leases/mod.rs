mod lease;

pub use lease::ModuleLease;

pub(crate) use lease::LeaseRequests;

#[cfg(test)]
#[path = "tests/leases.rs"]
mod leases_tests;
