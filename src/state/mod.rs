mod access;
mod acquisition;
mod controller;
pub(crate) mod locales;

pub use access::{LocaleView, LocaleViewMut};
pub use controller::Localization;

pub(crate) use acquisition::Acquisition;
pub(crate) use controller::RequestedModule;

#[cfg(test)]
#[path = "tests/state.rs"]
mod state_tests;

#[cfg(test)]
#[path = "tests/locales.rs"]
mod locale_tests;
