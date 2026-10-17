mod locale;

pub use locale::{CommitLocaleError, PreparationStatus};

pub(crate) use locale::commit;
#[cfg(feature = "manifest")]
pub(crate) use locale::{load_asset, next_asset_request, refresh_handoff};

#[cfg(test)]
#[path = "tests/preparation.rs"]
mod preparation_tests;

#[cfg(test)]
#[path = "tests/switching.rs"]
mod switching_tests;
