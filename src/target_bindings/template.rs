use crate::bevy::ecs::{
	error::Result,
	template::{FromTemplate, Template, TemplateContext},
};
use crate::{FluentScope, LocalizationTarget, Localized, Msg};
use std::{error::Error, fmt, marker::PhantomData};

#[doc(hidden)]
pub struct LocalizedTemplate<T: LocalizationTarget, S: FluentScope> {
	message: Option<Msg<S>>,
	marker: PhantomData<fn() -> T>,
}

impl<T: LocalizationTarget, S: FluentScope> Default for LocalizedTemplate<T, S> {
	fn default() -> Self {
		Self {
			message: None,
			marker: PhantomData,
		}
	}
}

impl<T: LocalizationTarget, S: FluentScope> LocalizedTemplate<T, S> {
	/// Capture a formatter without reading or requesting the catalog.
	pub fn new(format: impl Fn(&S) -> String + Send + Sync + 'static) -> Self {
		Self::from(Msg::new(format))
	}
}

impl<T: LocalizationTarget, S: FluentScope> From<Msg<S>> for LocalizedTemplate<T, S> {
	fn from(message: Msg<S>) -> Self {
		Self {
			message: Some(message),
			marker: PhantomData,
		}
	}
}

impl<T: LocalizationTarget, S: FluentScope> From<Localized<T, S>> for LocalizedTemplate<T, S> {
	fn from(binding: Localized<T, S>) -> Self {
		Self::from(binding.message)
	}
}

impl<T: LocalizationTarget, S: FluentScope> Template for LocalizedTemplate<T, S> {
	type Output = Localized<T, S>;

	fn build_template(&self, _: &mut TemplateContext) -> Result<Self::Output> {
		let Some(message) = &self.message else {
			return Err(MissingFormatter.into());
		};

		Ok(Localized::from(message.clone()))
	}

	fn clone_template(&self) -> Self {
		Self {
			message: self.message.clone(),
			marker: PhantomData,
		}
	}
}

impl<T: LocalizationTarget, S: FluentScope> FromTemplate for Localized<T, S> {
	type Template = LocalizedTemplate<T, S>;
}

#[derive(Debug)]
struct MissingFormatter;

impl fmt::Display for MissingFormatter {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str("Localized requires a formatting closure or Msg: use Localized::new(|scope| ...) or Localized::from(message)")
	}
}

impl Error for MissingFormatter {}
