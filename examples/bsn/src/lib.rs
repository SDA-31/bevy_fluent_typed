mod app;
mod buttons;
mod scene;

pub use app::app;
pub use buttons::{button, caption_button, caption_list_button};
pub use scene::greeting_scene;

bevy_fluent_typed::translations!(pub mod texts);

#[cfg(test)]
mod tests;
