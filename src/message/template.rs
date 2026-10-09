//! Native BSN constructors keep deferred formatting inside the canonical template.
use super::{LocalizedText, Message};
use crate::FluentScope;
use crate::bevy::ecs::{
	error::Result,
	template::{FromTemplate, Template, TemplateContext},
};
use std::{error::Error, fmt};

// This builder deliberately implements Template::clone_template rather than
// Clone: Bevy's blanket Template implementation for Clone types would otherwise
// compete with the custom output type on 0.20.
#[doc(hidden)]
pub struct LocalizedTextTemplate<S: FluentScope>(Option<Message<S>>);

impl<S: FluentScope> Default for LocalizedTextTemplate<S> {
	fn default() -> Self {
		Self(None)
	}
}

impl<S: FluentScope> LocalizedTextTemplate<S> {
	/// Capture a formatter without reading or requesting its catalog.
	pub fn new(format: impl Fn(&S) -> String + Send + Sync + 'static) -> Self {
		Self(Some(Message::new(format)))
	}
}

impl<S: FluentScope> From<Message<S>> for LocalizedTextTemplate<S> {
	fn from(message: Message<S>) -> Self {
		Self(Some(message))
	}
}

impl<S: FluentScope> From<LocalizedText<S>> for LocalizedTextTemplate<S> {
	fn from(binding: LocalizedText<S>) -> Self {
		Self(Some(binding.0))
	}
}

impl<S: FluentScope> Template for LocalizedTextTemplate<S> {
	type Output = LocalizedText<S>;

	fn build_template(&self, _: &mut TemplateContext) -> Result<Self::Output> {
		let Some(message) = &self.0 else {
			return Err(MissingFormatter.into());
		};

		Ok(LocalizedText::from(message.clone()))
	}

	fn clone_template(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<S: FluentScope> FromTemplate for LocalizedText<S> {
	type Template = LocalizedTextTemplate<S>;
}

#[derive(Debug)]
struct MissingFormatter;

impl fmt::Display for MissingFormatter {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(
			"LocalizedText requires a message: use LocalizedText::new(...) or LocalizedText::from(...)",
		)
	}
}

impl Error for MissingFormatter {}
