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

pub(crate) struct Consumer {
	root: TypeId,
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

pub(crate) type Consumers = Arc<[Arc<Consumer>]>;

#[derive(Resource, Default)]
struct Demands {
	consumers: Vec<Weak<Consumer>>,
	paths: HashMap<TypeId, Arc<BTreeSet<&'static str>>>,
	dirty: Arc<AtomicBool>,
	revision: u64,
}

impl Demands {
	fn synchronize(&mut self) {
		if !self.dirty.swap(false, Ordering::AcqRel) {
			return;
		}

		let mut paths: HashMap<TypeId, BTreeSet<&'static str>> = HashMap::new();
		self.consumers.retain(|owner| {
			let Some(owner) = owner.upgrade() else {
				return false;
			};

			if !owner.active.load(Ordering::Acquire) {
				return false;
			}

			paths
				.entry(owner.root)
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
		paths: S::module_paths(),
		active: AtomicBool::new(true),
		dirty: demands.dirty.clone(),
	});
	demands.consumers.push(Arc::downgrade(&consumer));
	demands.dirty.store(true, Ordering::Release);
	consumer
}

pub(crate) fn synchronize<C: FluentCatalog, M: LoadingMode>(world: &mut World) {
	if !M::AUTOMATIC {
		return;
	}

	world.flush();
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
		.get(&TypeId::of::<C>())
		.cloned()
		.unwrap_or_default();
	world
		.resource_mut::<Localization<C, M>>()
		.set_automatic(paths, revision);
}
