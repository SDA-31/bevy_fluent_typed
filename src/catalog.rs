//! Provider contracts for independently parsed leaves and complete typed scopes.
use crate::bevy::prelude::Resource;
use crate::{ModuleStore, ScopeRegistration};
use std::{
	any::{Any, TypeId},
	sync::Arc,
};

pub(crate) type SharedScope = Arc<dyn Any + Send + Sync>;
type Parser<C> =
	dyn Fn(<C as FluentCatalog>::Locale, &[u8]) -> Result<SharedScope, String> + Send + Sync;

/// A checked leaf parser associated with a logical FTL path.
pub struct Module<C: FluentCatalog> {
	/// Path below a language directory, e.g. `ui/menu.ftl`.
	pub path: &'static str,
	pub(crate) scope: TypeId,
	pub(crate) parse: Arc<Parser<C>>,
}

impl<C: FluentCatalog> Module<C> {
	/// Describe a leaf and its checked parser, without loading any source bytes.
	pub fn new<S: FluentScope<Catalog = C>>(
		path: &'static str,
		parse: impl Fn(C::Locale, &[u8]) -> Result<S, String> + Send + Sync + 'static,
	) -> Self {
		Self {
			path,
			scope: TypeId::of::<S>(),
			parse: Arc::new(move |locale, bytes| {
				parse(locale, bytes).map(|value| Arc::new(value) as SharedScope)
			}),
		}
	}
}

/// One generated root, directory group or independently loadable leaf.
///
/// Providers are immutable snapshots. Cloning should share parsed resources;
/// generated scopes use `Arc`. Assembly must not read files or reparse Fluent.
pub trait FluentScope: Resource + Clone {
	/// Root provider owning this scope's locale and module schema.
	type Catalog: FluentCatalog;

	/// All logical leaves required to make this scope complete.
	fn module_paths() -> &'static [&'static str];

	/// Assemble a complete scope from ready children, or return `None`.
	/// Leaf implementations clone their existing value from `modules`.
	fn assemble(modules: &ModuleStore<Self::Catalog>) -> Option<Self>;
}

/// Root provider connecting compiled typed schemas to the Bevy runtime.
///
/// Generation is optional. Handwritten providers implement the same scope and
/// checked-leaf contracts. Sources are supplied separately as readable bytes, an
/// asynchronous loader, or an optional manifest.
pub trait FluentCatalog: FluentScope<Catalog = Self> {
	/// Stable compiled locale code; identifiers must be unique directory names.
	type Locale: Copy + Eq + Send + Sync + AsRef<str> + 'static;

	/// Typed navigation over the schema, including partially loaded trees.
	type Modules<'a>
	where
		Self: 'a;

	/// All known locales in a stable nonempty order.
	fn locales() -> &'static [Self::Locale];

	/// Startup language, present in `locales`.
	fn default_locale() -> Self::Locale;

	/// Locale whose annotations define the compiled schema.
	fn source_locale() -> Self::Locale {
		Self::default_locale()
	}

	/// Checked parsers for all unique logical leaves; no source bytes or I/O.
	fn modules() -> Vec<Module<Self>>;

	/// Root, groups and leaves, ordered with parents before their children.
	fn scopes() -> Vec<ScopeRegistration<Self>>;

	/// Construct the schema-navigation view without triggering loading.
	fn view(modules: &ModuleStore<Self>) -> Self::Modules<'_>;
}
