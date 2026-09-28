//! Assemble complete scopes from ready leaves and synchronize only changed resources.
use crate::bevy::prelude::*;
use crate::scope::ReadyScope;
use crate::{FluentCatalog, LoadingMode, Localization};

pub(crate) fn synchronize<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	// Bevy 0.19 reinserts a scoped resource with the current change tick even
	// when its Mut was never written. Avoid entering the scope on idle frames.
	if !needs_sync::<C, M>(world) {
		return;
	}

	world.resource_scope(|world, mut localization: Mut<Localization<C, M>>| {
		let scopes = localization.store.scopes.clone();

		for scope in scopes.iter().rev() {
			let Some(signature) = localization.store.signature(scope.paths) else {
				if localization.store.values.contains_key(&scope.id) {
					localization.store.values.remove(&scope.id);
				}

				if localization.published.contains_key(&scope.id) {
					localization.published.remove(&scope.id);
				}
				(scope.remove)(world);
				continue;
			};
			let changed = localization
				.store
				.values
				.get(&scope.id)
				.is_none_or(|value| value.signature != signature);

			if changed {
				let Some(value) = (scope.assemble)(&localization.store) else {
					continue;
				};
				localization.store.revision += 1;
				let revision = localization.store.revision;
				localization.store.values.insert(
					scope.id,
					ReadyScope {
						value,
						signature,
						revision,
					},
				);
			}

			let Some(value) = localization.store.values.get(&scope.id) else {
				continue;
			};
			let revision = value.revision;

			if localization.published.get(&scope.id) != Some(&revision) || !(scope.exists)(world) {
				(scope.publish)(&value.value, world);
				localization.published.insert(scope.id, revision);
			}
		}

		localization.synchronized = localization.store.revision;
	});
}

fn needs_sync<C: FluentCatalog, M: LoadingMode>(world: &World) -> bool {
	let localization = world.resource::<Localization<C, M>>();

	if localization.synchronized != localization.store.revision {
		return true;
	}

	// Resource removal by the host still restores the published snapshot. Idle
	// frames inspect only published scopes, without rebuilding leaf signatures.
	localization.published.keys().any(|id| {
		let scope = &localization.store.scopes[localization.store.scope_indices[id]];
		!(scope.exists)(world)
	})
}
