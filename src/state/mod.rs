mod access;
mod acquisition;
mod controller;
mod locales;
mod snapshots;

pub use access::{LocaleAccess, LocaleAccessMut};
pub use controller::Localization;

pub(crate) use acquisition::Acquisition;
pub(crate) use controller::RequestedModule;
pub(crate) use locales::LocaleData;
pub(crate) use snapshots::{LocaleSnapshot, LocaleSnapshots, SnapshotStamp};

#[cfg(test)]
#[path = "tests/state.rs"]
mod state_tests;

#[cfg(test)]
#[path = "tests/locales.rs"]
mod locale_tests;
