mod backend;
#[cfg(any(feature = "bevy-0-16", feature = "bevy-0-17", feature = "bevy-0-18"))]
pub(crate) mod readiness;
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
pub(crate) mod scheduling;

pub(crate) use backend::{
	HookContext, MessageReader, MessageWriter, Tick, UiSystems, after_accessibility_layout,
	before_text_detection, register_notifications, update_text2d_layout, write_update,
};
#[cfg(feature = "manifest")]
pub(crate) use backend::{asset_path, load_with_settings};
