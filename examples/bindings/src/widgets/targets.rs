use bevy_fluent_typed::bevy::prelude::{Component, EntityWorldMut, Mut, Without};
use bevy_fluent_typed::{LocalizationTarget, LocalizationUpdate, TargetItem, UpdateResult};

/// Store only the translated output; Localized owns the deferred message.
#[derive(Component, Default)]
pub(crate) struct Caption {
	pub text: String,
}

/// A second output on the same entity, synchronized by the same adapter.
#[derive(Component, Default)]
pub(crate) struct CaptionMirror(pub String);

/// Temporarily exclude this entity from Caption's localization query.
#[derive(Component)]
pub(crate) struct CaptionPaused;

impl LocalizationTarget for Caption {
	type Data = (Mut<'static, Self>, Mut<'static, CaptionMirror>);
	type Filter = Without<CaptionPaused>;

	fn initialize(entity: &mut EntityWorldMut<'_>) {
		if !entity.contains::<Self>() {
			entity.insert(Self::default());
		}

		if !entity.contains::<CaptionMirror>() {
			entity.insert(CaptionMirror::default());
		}
	}

	fn needs_update(
		(caption, mirror): &TargetItem<'_, '_, Self>,
		update: &LocalizationUpdate<'_>,
	) -> bool {
		update.is_changed() || update.component_changed(caption) || update.component_changed(mirror)
	}

	fn update(
		(caption, mut mirror): TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		// A mirror-only edit can be repaired from Caption without invoking Msg.
		if !update.is_changed() && !update.component_changed(&caption) {
			if mirror.0 != caption.text {
				caption.text.clone_into(&mut mirror.0);
			}

			return UpdateResult::Done;
		}

		update.set_text(caption.map_unchanged(|caption| &mut caption.text));
		update.set_text(mirror.map_unchanged(|mirror| &mut mirror.0));

		UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(caption) = entity.get_mut::<Self>() else {
			return;
		};

		update.set_text(caption.map_unchanged(|caption| &mut caption.text));

		let Some(mirror) = entity.get_mut::<CaptionMirror>() else {
			return;
		};

		update.set_text(mirror.map_unchanged(|mirror| &mut mirror.0));
	}
}

/// A single-field component implements the same public target contract.
#[derive(Component, Default)]
pub(crate) struct Badge {
	pub text: String,
}

impl LocalizationTarget for Badge {
	type Data = Mut<'static, Self>;
	type Filter = ();

	fn initialize(entity: &mut EntityWorldMut<'_>) {
		if entity.contains::<Self>() {
			return;
		}

		entity.insert(Self::default());
	}

	fn needs_update(data: &TargetItem<'_, '_, Self>, update: &LocalizationUpdate<'_>) -> bool {
		update.is_changed() || update.component_changed(data)
	}

	fn update(
		target: TargetItem<'_, '_, Self>,
		update: &mut LocalizationUpdate<'_>,
	) -> UpdateResult {
		update.set_text(target.map_unchanged(|target| &mut target.text));

		UpdateResult::Done
	}

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(target) = entity.get_mut::<Self>() else {
			return;
		};

		update.set_text(target.map_unchanged(|target| &mut target.text));
	}
}

/// This component has two independent fields and does not store any Msg.
#[derive(Component)]
pub(crate) struct Card {
	pub title: String,
	pub detail: String,
}

pub(crate) struct CardTitle;

impl LocalizationTarget for CardTitle {
	type Data = &'static mut Card;
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(mut card) = entity.get_mut::<Card>() else {
			return;
		};
		let text = update.text();

		if card.title != text {
			text.clone_into(&mut card.title);
		}
	}
}

pub(crate) struct CardDetail;

impl LocalizationTarget for CardDetail {
	type Data = &'static mut Card;
	type Filter = ();

	fn apply(entity: &mut EntityWorldMut<'_>, update: &mut LocalizationUpdate<'_>) {
		let Some(mut card) = entity.get_mut::<Card>() else {
			return;
		};
		let text = update.text();

		if card.detail != text {
			text.clone_into(&mut card.detail);
		}
	}
}
