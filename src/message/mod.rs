mod deferred;

pub use deferred::{CatalogUpdate, CatalogUpdateReader, Msg, ReloadCatalogs};

pub(crate) use deferred::MsgIdentity;

#[allow(
	deprecated,
	reason = "Preserve the compatibility alias in its module facade"
)]
pub use deferred::Message;

#[cfg(test)]
#[path = "tests/msg.rs"]
mod msg_tests;
