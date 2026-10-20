//! Mode-independent ready snapshots used by the existing target updater.
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{Resource, World},
};
use crate::catalog::SharedScope;
use crate::{FluentCatalog, FluentScope};
use std::{num::NonZeroU64, sync::Arc};

/// Nonzero ready identity lets absent snapshots use no extra per-binding word.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotStamp(NonZeroU64);

impl SnapshotStamp {
	/// Preserve every Bevy tick, including zero, while reserving zero for absence.
	pub(crate) fn from_change_tick(tick: u32) -> Self {
		Self(NonZeroU64::new(u64::from(tick) + 1).expect("shifted u32 tick is nonzero"))
	}
}

/// One immutable ready scope; identity changes only when its source snapshot changes.
#[derive(Clone)]
pub(crate) struct LocaleSnapshot<S: FluentScope> {
	pub(crate) locale: <S::Catalog as FluentCatalog>::Locale,
	pub(crate) value: S,
	pub(crate) stamp: SnapshotStamp,
	source: SharedScope,
}

/// Read-only snapshot table independent of the provider's loading mode.
#[derive(Resource)]
pub(crate) struct LocaleSnapshots<S: FluentScope> {
	pub(crate) ready: Arc<Vec<LocaleSnapshot<S>>>,
	next: u64,
}

impl<S: FluentScope> LocaleSnapshots<S> {
	/// Publication identity for the whole table, including removed entries.
	pub(crate) fn version(&self) -> u64 {
		self.next
	}

	pub(crate) fn publish(
		world: &mut World,
		locale: <S::Catalog as FluentCatalog>::Locale,
		value: Option<&SharedScope>,
	) {
		let current = world.get_resource::<Self>().and_then(|table| {
			table
				.ready
				.iter()
				.find(|snapshot| snapshot.locale == locale)
		});
		let same = match (current, value) {
			(Some(current), Some(value)) => Arc::ptr_eq(&current.source, value),
			(None, None) => true,
			_ => false,
		};

		if same {
			return;
		}

		if !world.contains_resource::<Self>() {
			world.insert_resource(Self {
				ready: Arc::new(Vec::new()),
				next: 0,
			});
		}

		let mut table = world.resource_mut::<Self>();
		// Keep zero available for Option<SnapshotStamp>'s absent representation.
		table.next = table.next.wrapping_add(1).max(1);
		let stamp = SnapshotStamp(NonZeroU64::new(table.next).expect("publication is nonzero"));
		let rows = Arc::make_mut(&mut table.ready);
		rows.retain(|snapshot| snapshot.locale != locale);

		if let Some(value) = value {
			rows.push(LocaleSnapshot {
				locale,
				value: value
					.downcast_ref::<S>()
					.expect("typed scope snapshot")
					.clone(),
				stamp,
				source: value.clone(),
			});
		}
	}
}

#[cfg(test)]
#[path = "tests/snapshots.rs"]
mod tests;
