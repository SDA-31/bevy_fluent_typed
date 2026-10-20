mod diagnostics;

pub use diagnostics::LocalizationRegistrationError;

pub(crate) use diagnostics::{RegistrationSite, validate};

#[cfg(test)]
#[path = "tests/registration.rs"]
mod registration_tests;
