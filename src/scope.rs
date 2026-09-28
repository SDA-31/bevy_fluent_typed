//! Immutable scope storage and type-erased publication hooks for generated providers.
use crate::bevy::ecs::world::{FilteredResources, FilteredResourcesBuilder};
use crate::bevy::prelude::*;
use crate::catalog::SharedScope;
use crate::{FluentCatalog, FluentScope, ModuleError, ModuleStatus, bindings};
use std::{
	any::TypeId,
	collections::{BTreeMap, HashMap},
	sync::Arc,
};

pub(crate) struct ReadyScope {
	pub(crate) value: SharedScope,
	pub(crate) signature: Vec<(&'static str, u64)>,
	pub(crate) revision: u64,
}

/// Read-only access to ready scopes for typed navigation and provider assembly.
pub struct ModuleStore<C: FluentCatalog> {
	pub(crate) locale: C::Locale,
	pub(crate) values: HashMap<TypeId, ReadyScope>,
	pub(crate) states: BTreeMap<&'static str, ModuleStatus>,
	pub(crate) leaves: BTreeMap<&'static str, TypeId>,
	pub(crate) scopes: Arc<[ScopeRegistration<C>]>,
	pub(crate) scope_indices: HashMap<TypeId, usize>,
	pub(crate) revision: u64,
}

impl<C: FluentCatalog> ModuleStore<C> {
	pub(crate) fn new(locale: C::Locale) -> Self {
		let scopes: Arc<[ScopeRegistration<C>]> = C::scopes().into();
		let scope_indices = scopes
			.iter()
			.enumerate()
			.map(|(index, scope)| (scope.id, index))
			.collect();

		Self {
			locale,
			values: HashMap::new(),
			states: BTreeMap::new(),
			leaves: C::modules()
				.into_iter()
				.map(|module| (module.path, module.scope))
				.collect(),
			revision: 0,
			scopes,
			scope_indices,
		}
	}

	/// Selected locale, including while its modules are still loading.
	pub fn locale(&self) -> C::Locale {
		self.locale
	}

	/// Borrow a complete scope without requesting or loading it.
	///
	/// # Errors
	/// Reports the first unavailable required leaf and its latest loading state.
	pub fn get<S: FluentScope<Catalog = C>>(&self) -> Result<&S, ModuleError> {
		if let Some(value) = self
			.values
			.get(&TypeId::of::<S>())
			.and_then(|value| value.value.downcast_ref())
		{
			return Ok(value);
		}

		let path = S::module_paths()
			.iter()
			.copied()
			.find(|path| {
				self.leaves
					.get(path)
					.is_none_or(|id| !self.values.contains_key(id))
			})
			.unwrap_or_else(|| S::module_paths().first().copied().unwrap_or(""));
		Err(ModuleError {
			locale: self.locale.as_ref().into(),
			path,
			status: self
				.states
				.get(path)
				.cloned()
				.unwrap_or(ModuleStatus::Unloaded),
		})
	}

	/// Latest attempt status across a scope's leaves; availability is queried with `get`.
	pub fn status<S: FluentScope<Catalog = C>>(&self) -> ModuleStatus {
		let mut result = ModuleStatus::Ready;

		for path in S::module_paths() {
			match self
				.states
				.get(path)
				.cloned()
				.unwrap_or(ModuleStatus::Unloaded)
			{
				failed @ ModuleStatus::Failed(_) => return failed,
				ModuleStatus::Unloaded => return ModuleStatus::Unloaded,
				ModuleStatus::Loading => result = ModuleStatus::Loading,
				ModuleStatus::Ready => {}
			}
		}

		result
	}

	pub(crate) fn insert_leaf(&mut self, path: &'static str, value: SharedScope) {
		self.revision += 1;
		self.values.insert(
			self.leaves[path],
			ReadyScope {
				value,
				signature: vec![(path, self.revision)],
				revision: self.revision,
			},
		);
		self.states.insert(path, ModuleStatus::Ready);
	}

	pub(crate) fn signature(&self, paths: &[&'static str]) -> Option<Vec<(&'static str, u64)>> {
		paths
			.iter()
			.map(|path| {
				self.values
					.get(self.leaves.get(path)?)
					.map(|value| (*path, value.revision))
			})
			.collect()
	}
}

/// Type-erased registration for one root, group or leaf resource.
/// Provider integrations construct these with `new`; applications use typed resources.
pub struct ScopeRegistration<C: FluentCatalog> {
	pub(crate) id: TypeId,
	pub(crate) parameter: TypeId,
	pub(crate) paths: &'static [&'static str],
	pub(crate) assemble: fn(&ModuleStore<C>) -> Option<SharedScope>,
	pub(crate) publish: fn(&SharedScope, &mut World),
	pub(crate) remove: fn(&mut World),
	pub(crate) exists: fn(&World) -> bool,
	pub(crate) ready_access: fn(&mut FilteredResourcesBuilder),
	pub(crate) ready_exists: fn(&FilteredResources) -> bool,
	pub(crate) bindings: fn(&mut App),
}

impl<C: FluentCatalog> ScopeRegistration<C> {
	/// Register one scope's assembly, resources and deferred text bindings.
	pub fn new<S: FluentScope<Catalog = C>>() -> Self {
		Self {
			id: TypeId::of::<S>(),
			parameter: TypeId::of::<Res<'static, S>>(),
			paths: S::module_paths(),
			assemble: |store| S::assemble(store).map(|scope| Arc::new(scope) as SharedScope),
			publish: |scope, world| {
				world.insert_resource(
					scope
						.downcast_ref::<S>()
						.expect("provider scope type")
						.clone(),
				);
			},
			remove: |world| {
				world.remove_resource::<S>();
			},
			exists: |world| world.contains_resource::<S>(),
			ready_access: |builder| {
				builder.add_read::<S>();
			},
			ready_exists: |resources| resources.get::<S>().is_ok(),
			bindings: bindings::register::<S>,
		}
	}
}
