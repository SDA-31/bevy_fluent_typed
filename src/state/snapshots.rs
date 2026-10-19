//! Mode-independent ready snapshots used by the existing target updater.
use crate::bevy::{
	ecs as bevy_ecs,
	prelude::{Resource, World},
};
use crate::catalog::SharedScope;
use crate::{FluentCatalog, FluentScope};
use std::sync::Arc;

/// One immutable ready scope; identity changes only when its source snapshot changes.
#[derive(Clone)]
pub(crate) struct LocaleSnapshot<S: FluentScope> {
	pub(crate) locale: <S::Catalog as FluentCatalog>::Locale,
	pub(crate) value: S,
	pub(crate) stamp: u64,
	source: SharedScope,
}

/// Read-only snapshot table independent of the provider's loading mode.
#[derive(Resource)]
pub(crate) struct LocaleSnapshots<S: FluentScope> {
	pub(crate) ready: Arc<Vec<LocaleSnapshot<S>>>,
	next: u64,
}

impl<S: FluentScope> LocaleSnapshots<S> {
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
		table.next = table.next.wrapping_add(1);
		let stamp = table.next;
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
