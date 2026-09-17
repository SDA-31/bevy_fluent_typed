//! Keep Bevy minor-version differences out of loading and scheduling policy.
use crate::bevy::{
	asset::{AssetPath, LoadContext},
	ecs::{schedule::ScheduleConfigs, system::ScheduleSystem},
	prelude::*,
};

pub(crate) fn asset_path<'a>(context: &'a LoadContext<'_>) -> &'a AssetPath<'static> {
	#[cfg(all(
		feature = "bevy-0-17",
		not(any(feature = "bevy-0-18", feature = "bevy-0-19"))
	))]
	{
		context.asset_path()
	}

	#[cfg(any(feature = "bevy-0-18", feature = "bevy-0-19"))]
	{
		context.path()
	}
}

pub(crate) fn before_text_detection<M>(
	systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
) -> ScheduleConfigs<ScheduleSystem> {
	#[cfg(feature = "bevy-0-19")]
	{
		systems.before(crate::bevy::text::detect_text_needs_rerender)
	}

	#[cfg(not(feature = "bevy-0-19"))]
	{
		systems
			.before(crate::bevy::text::detect_text_needs_rerender::<Text>)
			.before(crate::bevy::text::detect_text_needs_rerender::<Text2d>)
	}
}
