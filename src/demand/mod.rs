mod tracking;

pub(crate) use tracking::{Consumer, acquire, synchronize};

#[cfg(test)]
#[path = "tests/automatic.rs"]
mod automatic_tests;
