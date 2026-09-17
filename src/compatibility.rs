//! Keep Bevy minor-version differences out of loading and scheduling policy.
use crate::bevy::{
	asset::{AssetPath, LoadContext},
	ecs::{schedule::ScheduleConfigs, system::ScheduleSystem},
	prelude::*,
};
use crate::{CatalogUpdate, FluentCatalog, ReloadCatalogs};

#[cfg(feature = "bevy-0-16")]
pub(crate) use crate::bevy::{
	prelude::{EventReader as MessageReader, EventWriter as MessageWriter},
	text::update_text2d_layout,
	ui::UiSystem as UiSystems,
};

#[cfg(not(feature = "bevy-0-16"))]
pub(crate) use crate::bevy::{
	prelude::{MessageReader, MessageWriter},
	sprite::update_text2d_layout,
	ui::UiSystems,
};

pub(crate) fn register_notifications<C: FluentCatalog>(app: &mut App) {
	#[cfg(feature = "bevy-0-16")]
	app.add_event::<CatalogUpdate<C>>()
		.add_event::<ReloadCatalogs<C>>();

	#[cfg(not(feature = "bevy-0-16"))]
	app.add_message::<CatalogUpdate<C>>()
		.add_message::<ReloadCatalogs<C>>();
}

pub(crate) fn asset_path<'a>(context: &'a LoadContext<'_>) -> &'a AssetPath<'static> {
	#[cfg(all(
		any(feature = "bevy-0-16", feature = "bevy-0-17"),
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
