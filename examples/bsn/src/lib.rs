mod app;
mod scene;

pub use app::app;
pub use scene::greeting_scene;

bevy_fluent_typed::translations!(pub mod texts);

#[cfg(test)]
mod tests;
