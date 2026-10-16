use crate::bevy::prelude::{DetectChanges, EntityWorldMut, Mut, Text, Text2d, TextSpan};
use crate::{LocalizationTarget, LocalizationUpdate, TargetItem, TargetReadItem, UpdateResult};
#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
use crate::{
	TargetUpdatePhase,
	bevy::prelude::{AccessibleLabel, Ref},
};

impl LocalizationTarget for Text {
	type Data = Mut<'static, Self>;
	type Filter = ();

	fn initialize(entity: &mut EntityWorldMut<'_>) {
		if !entity.contains::<Self>() {
			entity.insert(Self::default());
		}
	}

	#[inline]
	fn needs_update(data: &TargetItem<'_, '_, Self>, update: &LocalizationUpdate<'_>) -> bool {
		update.is_changed() || data.is_changed()
	}

	#[inline]
	fn update(
		target: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		update.set_text(target.map_unchanged(|target| &mut target.0));

		UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get_mut::<Self>() else {
			return;
		};
		update.set_text(target.map_unchanged(|target| &mut target.0));
	}

	fn matches(target: TargetReadItem<'_, '_, Self>, text: &str) -> bool {
		target.0 == text
	}
}

impl LocalizationTarget for Text2d {
	type Data = Mut<'static, Self>;
	type Filter = ();

	fn initialize(entity: &mut EntityWorldMut<'_>) {
		if !entity.contains::<Self>() {
			entity.insert(Self::default());
		}
	}

	#[inline]
	fn needs_update(data: &TargetItem<'_, '_, Self>, update: &LocalizationUpdate<'_>) -> bool {
		update.is_changed() || data.is_changed()
	}

	#[inline]
	fn update(
		target: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		update.set_text(target.map_unchanged(|target| &mut target.0));

		UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get_mut::<Self>() else {
			return;
		};
		update.set_text(target.map_unchanged(|target| &mut target.0));
	}

	fn matches(target: TargetReadItem<'_, '_, Self>, text: &str) -> bool {
		target.0 == text
	}
}

impl LocalizationTarget for TextSpan {
	type Data = Mut<'static, Self>;
	type Filter = ();

	fn initialize(entity: &mut EntityWorldMut<'_>) {
		if !entity.contains::<Self>() {
			entity.insert(Self::default());
		}
	}

	#[inline]
	fn needs_update(data: &TargetItem<'_, '_, Self>, update: &LocalizationUpdate<'_>) -> bool {
		update.is_changed() || data.is_changed()
	}

	#[inline]
	fn update(
		target: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		update.set_text(target.map_unchanged(|target| &mut target.0));

		UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get_mut::<Self>() else {
			return;
		};
		update.set_text(target.map_unchanged(|target| &mut target.0));
	}

	fn matches(target: TargetReadItem<'_, '_, Self>, text: &str) -> bool {
		target.0 == text
	}
}

#[cfg(any(feature = "bevy-0-19", feature = "bevy-0-20"))]
impl LocalizationTarget for AccessibleLabel {
	type Data = Ref<'static, Self>;
	type Filter = ();

	const PHASE: TargetUpdatePhase = TargetUpdatePhase::AfterLayout;

	fn initialize(entity: &mut EntityWorldMut<'_>) {
		if !entity.contains::<Self>() {
			entity.insert(Self::default());
		}
	}

	fn needs_update(data: &TargetItem<'_, '_, Self>, update: &LocalizationUpdate<'_>) -> bool {
		update.is_changed() || update.component_changed(data)
	}

	fn update(
		target: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		if target.0 == update.text() {
			return UpdateResult::Done;
		}

		UpdateResult::Deferred
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get::<Self>() else {
			return;
		};
		let text = update.text();

		if target.0 == text {
			return;
		}

		// Bevy owns AccessibilityNode and runs the ordinary label hooks.
		entity.insert(Self::new(text));
	}

	fn matches(target: TargetReadItem<'_, '_, Self>, text: &str) -> bool {
		target.0 == text
	}
}
