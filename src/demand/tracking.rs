//! World-owned consumer demand, independent of controller and catalog lifetimes.
use crate::bevy::{ecs as bevy_ecs, prelude::*};
use crate::{FluentCatalog, FluentScope, LoadingMode, Localization};
use std::{
	any::TypeId,
	collections::{BTreeSet, HashMap},
	sync::{
		Arc, Weak,
		atomic::{AtomicBool, Ordering},
	},
};

/// Lifetime token for one automatically retained scope.
/// Release and Drop are idempotent; a shared dirty flag invalidates provider unions.
pub(crate) struct Consumer {
	root: TypeId,
	locale: Option<usize>,
	paths: &'static [&'static str],
	active: AtomicBool,
	dirty: Arc<AtomicBool>,
}

impl Consumer {
	pub(crate) fn release(&self) {
		if self.active.swap(false, Ordering::AcqRel) {
			self.dirty.store(true, Ordering::Release);
		}
	}
}

impl Drop for Consumer {
	fn drop(&mut self) {
		self.release();
	}
}

/// World-owned weak registry of active consumer tokens.
/// Cached per-provider leaf unions rebuild only after ownership changes, without
/// extending system or entity lifetimes.
#[derive(Resource, Default)]
struct Demands {
	consumers: Vec<Weak<Consumer>>,
	paths: HashMap<(TypeId, Option<usize>), Arc<BTreeSet<&'static str>>>,
	bindings: HashMap<(TypeId, Option<usize>), Weak<Consumer>>,
	dirty: Arc<AtomicBool>,
	revision: u64,
}

impl Demands {
	fn synchronize(&mut self) {
		if !self.dirty.swap(false, Ordering::AcqRel) {
			return;
		}

		let mut paths: HashMap<(TypeId, Option<usize>), BTreeSet<&'static str>> = HashMap::new();
		self.consumers.retain(|owner| {
			let Some(owner) = owner.upgrade() else {
				return false;
			};

			if !owner.active.load(Ordering::Acquire) {
				return false;
			}

			paths
				.entry((owner.root, owner.locale))
				.or_default()
				.extend(owner.paths.iter().copied());
			true
		});
		self.paths = paths
			.into_iter()
			.map(|(root, paths)| (root, Arc::new(paths)))
			.collect();
		self.revision += 1;
	}
}

pub(crate) fn acquire<S: FluentScope>(world: &mut World) -> Arc<Consumer> {
	world.init_resource::<Demands>();
	let mut demands = world.resource_mut::<Demands>();
	let consumer = Arc::new(Consumer {
		root: TypeId::of::<S::Catalog>(),
		locale: None,
		paths: S::module_paths(),
		active: AtomicBool::new(true),
		dirty: demands.dirty.clone(),
	});
	demands.consumers.push(Arc::downgrade(&consumer));
	demands.dirty.store(true, Ordering::Release);
	consumer
}

pub(crate) fn acquire_binding<S: FluentScope>(
	world: &mut World,
	locale: Option<<S::Catalog as FluentCatalog>::Locale>,
) -> Arc<Consumer> {
	let locale = locale.map(|selected| {
		<S::Catalog as FluentCatalog>::locales()
			.iter()
			.position(|locale| *locale == selected)
			.expect("compiled locale")
	});
	let key = (TypeId::of::<S>(), locale);
	world.init_resource::<Demands>();

	if let Some(consumer) = world
		.resource::<Demands>()
		.bindings
		.get(&key)
		.and_then(Weak::upgrade)
	{
		return consumer;
	}

	let mut demands = world.resource_mut::<Demands>();
	let consumer = Arc::new(Consumer {
		root: TypeId::of::<S::Catalog>(),
		locale,
		paths: S::module_paths(),
		active: AtomicBool::new(true),
		dirty: demands.dirty.clone(),
	});
	demands.consumers.push(Arc::downgrade(&consumer));
	demands.bindings.insert(key, Arc::downgrade(&consumer));
	demands.dirty.store(true, Ordering::Release);
	consumer
}

pub(crate) fn synchronize<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	world.flush();

	if !M::AUTOMATIC && !M::FULL {
		return;
	}

	crate::bindings::synchronize_consumers::<C>(world);

	let Some(mut demands) = world.get_resource_mut::<Demands>() else {
		return;
	};
	demands.synchronize();
	let revision = demands.revision;
	let localization = world.resource::<Localization<C, M>>();

	if localization.automatic_revision == revision {
		return;
	}

	let paths = world
		.resource::<Demands>()
		.paths
		.get(&(TypeId::of::<C>(), None))
		.cloned()
		.unwrap_or_default();
	let fixed = C::locales()
		.iter()
		.enumerate()
		.map(|(index, _)| {
			world
				.resource::<Demands>()
				.paths
				.get(&(TypeId::of::<C>(), Some(index)))
				.cloned()
				.unwrap_or_default()
		})
		.collect::<Vec<_>>();
	let mut localization = world.resource_mut::<Localization<C, M>>();
	localization.set_automatic(paths, revision);
	localization.set_fixed_automatic(fixed);
}
