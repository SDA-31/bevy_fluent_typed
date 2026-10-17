mod localization;

pub use localization::{LocalizationAccessibilitySet, LocalizationPlugin, LocalizationSystems};

#[cfg(all(test, feature = "manifest"))]
#[path = "tests/lazy.rs"]
mod lazy_tests;
