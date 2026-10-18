//! Assemble complete scopes from ready leaves and synchronize only changed resources.
use crate::bevy::prelude::{Mut, World};
use crate::scope::ReadyScope;
use crate::{FluentCatalog, LoadingMode, Localization, ScopeRegistration};

pub(crate) fn synchronize<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	// Bevy 0.19 reinserts a scoped resource with the current change tick even
	// when its Mut was never written. Avoid entering the scope on idle frames.
	if !needs_sync::<C, M>(world) && !has_additional_changes::<C, M>(world) {
		return;
	}

	world.resource_scope(|world, mut localization: Mut<Localization<C, M>>| {
		if let Some(target) = localization.preparation.as_mut() {
			if target.synchronized != target.store.revision {
				assemble_store(&mut target.store);
				target.synchronized = target.store.revision;
			}
		}

		for slot in &mut localization.fixed {
			if let Some(data) = &mut slot.data {
				if data.synchronized != data.store.revision {
					assemble_store(&mut data.store);
					data.synchronized = data.store.revision;
				}
			}
		}

		let scopes = localization.store.scopes.clone();

		for scope in scopes.iter().rev() {
			synchronize_scope(world, &mut localization, scope);
		}

		localization.synchronized = localization.store.revision;
	});
}

fn synchronize_scope<C: FluentCatalog, M: LoadingMode>(
	world: &mut World,
	localization: &mut Localization<C, M>,
	scope: &ScopeRegistration<C>,
) {
	if !assemble_scope(&mut localization.store, scope) {
		return;
	}

	if !localization.store.values.contains_key(&scope.id) {
		localization.published.remove(&scope.id);
		(scope.remove)(world);
		return;
	}

	let Some(value) = localization.store.values.get(&scope.id) else {
		return;
	};
	let revision = value.revision;

	if localization.published.get(&scope.id) != Some(&revision) || !(scope.exists)(world) {
		(scope.publish)(&value.value, world);
		localization.published.insert(scope.id, revision);
	}
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

/// Build ready scopes without publishing them as native resources.
pub(crate) fn assemble_store<C: FluentCatalog>(store: &mut crate::ModuleStore<C>) {
	let scopes = store.scopes.clone();

	for scope in scopes.iter().rev() {
		assemble_scope(store, scope);
	}
}

fn assemble_scope<C: FluentCatalog>(
	store: &mut crate::ModuleStore<C>,
	scope: &ScopeRegistration<C>,
) -> bool {
	let Some(signature) = store.signature(scope.paths) else {
		store.values.remove(&scope.id);
		return true;
	};

	if store
		.values
		.get(&scope.id)
		.is_some_and(|value| value.signature == signature)
	{
		return true;
	}

	let Some(value) = (scope.assemble)(store) else {
		return false;
	};
	store.revision += 1;
	let revision = store.revision;
	store.values.insert(
		scope.id,
		ReadyScope {
			value,
			signature,
			revision,
		},
	);
	true
}

fn has_additional_changes<C: FluentCatalog, M: LoadingMode>(world: &World) -> bool {
	let state = world.resource::<Localization<C, M>>();
	state
		.preparation
		.as_ref()
		.is_some_and(|target| target.synchronized != target.store.revision)
		|| state.fixed.iter().any(|slot| {
			slot.data
				.as_ref()
				.is_some_and(|data| data.synchronized != data.store.revision)
		})
}
