//! Structural plugin conflicts, independent of translation sources and locales.
use crate::bevy::prelude::App;
use crate::{FluentCatalog, progress};
use std::{
	any::{TypeId, type_name},
	fmt,
	panic::Location,
};

/// Two localization plugins claim the same root catalog type.
///
/// Every [`crate::LocalizationPlugin`] registers its complete root schema, even
/// when its source contains only a subset of leaves. Sources, languages and
/// translation bytes do not participate in this structural check.
/// Obtain this error through [`crate::LocalizationPlugin::validate_registration`].
/// Ordinary Bevy `add_plugins` reports it as a formatted panic before installing
/// the second plugin's localization resources or systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizationRegistrationError {
	/// Rust type identity of the conflicting root catalog.
	pub catalog: TypeId,
	/// Readable Rust type name of the conflicting root catalog.
	pub catalog_name: &'static str,
	/// Sorted, unique FTL leaf paths declared by the root schema.
	/// An empty list still represents a conflict of the root itself.
	pub modules: Vec<&'static str>,
	/// Concrete type of the first plugin, including its loading mode.
	pub first_plugin: &'static str,
	/// Location where the first plugin value was constructed.
	pub first_created_at: &'static Location<'static>,
	/// Concrete type of the incoming plugin, including its loading mode.
	pub second_plugin: &'static str,
	/// Constructor call site of the incoming plugin.
	pub second_created_at: &'static Location<'static>,
}

impl fmt::Display for LocalizationRegistrationError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			formatter,
			"conflicting localization plugin registrations for catalog `{}`\n  first: `{}` created at {}\n  second: `{}` created at {}",
			self.catalog_name,
			self.first_plugin,
			self.first_created_at,
			self.second_plugin,
			self.second_created_at,
		)?;

		for module in &self.modules {
			write!(formatter, "\n  conflicting module: {module}")?;
		}

		Ok(())
	}
}

impl std::error::Error for LocalizationRegistrationError {}

/// Constructor provenance retained for structural duplicate-provider diagnostics.
#[derive(Clone, Copy)]
pub(crate) struct RegistrationSite {
	pub(crate) plugin: &'static str,
	pub(crate) created_at: &'static Location<'static>,
}

pub(crate) fn validate<C: FluentCatalog>(
	app: &App,
	incoming: RegistrationSite,
) -> Result<(), LocalizationRegistrationError> {
	let Some(existing) = app.world().get_resource::<progress::Registration<C>>() else {
		return Ok(());
	};

	let mut modules = C::module_paths().to_vec();
	modules.sort_unstable();
	modules.dedup();

	Err(LocalizationRegistrationError {
		catalog: TypeId::of::<C>(),
		catalog_name: type_name::<C>(),
		modules,
		first_plugin: existing.site.plugin,
		first_created_at: existing.site.created_at,
		second_plugin: incoming.plugin,
		second_created_at: incoming.created_at,
	})
}
