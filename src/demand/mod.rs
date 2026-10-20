mod tracking;

pub(crate) use tracking::{Consumer, acquire, acquire_binding, synchronize};

#[cfg(test)]
#[path = "tests/automatic.rs"]
mod automatic_tests;
