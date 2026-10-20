use crate::bevy::prelude::{DetectChanges, DetectChangesMut, Mut};
use crate::compatibility::Tick;
use std::borrow::Cow;

/// The message value and invalidation signal for one adapter invocation.
///
/// [`Self::is_changed`] covers catalog/readiness changes, a different Msg, and
/// first entry or reentry into the adapter's query. Combine it with native
/// component change detection when deciding whether a target needs translation.
/// `component_changed` excludes confirmed own writes, including deferred work.
/// With [`crate::CachePolicy::None`], [`Self::text`] formats lazily, at most once
/// during this invocation. With `Rendered`, it reads the prepared cached value.
/// If the catalog is unavailable, the value is empty and no formatter is called.
pub struct LocalizationUpdate<'a> {
	changed: bool,
	/// Value obtained for this invocation when a retained cache is not borrowed.
	text: Option<Cow<'a, str>>,
	/// On-demand formatter used when output retention is disabled.
	render: Option<&'a dyn Fn() -> String>,
	/// Borrowed retained output owned by the binding's ECS state.
	cache: Option<&'a mut String>,
	/// New output awaiting installation into the retained cache or target buffer.
	fresh: Option<String>,
	/// Last confirmed adapter tick, used to exclude its own writes.
	observed: Option<u32>,
	this_run: u32,
	/// Whether this invocation consumed the translation, including an empty value.
	read: bool,
}

impl<'a> LocalizationUpdate<'a> {
	pub(super) fn cached(changed: bool, text: &'a str) -> Self {
		Self {
			changed,
			text: Some(Cow::Borrowed(text)),
			render: None,
			cache: None,
			fresh: None,
			observed: None,
			this_run: 0,
			read: false,
		}
	}

	pub(super) fn cached_update(
		changed: bool,
		cache: &'a mut String,
		fresh: Option<String>,
	) -> Self {
		Self {
			cache: Some(cache),
			fresh,
			..Self::cached(changed, "")
		}
	}

	pub(super) fn lazy(changed: bool, render: &'a dyn Fn() -> String) -> Self {
		Self {
			changed,
			text: None,
			render: Some(render),
			cache: None,
			fresh: None,
			observed: None,
			this_run: 0,
			read: false,
		}
	}

	pub(super) fn owned(changed: bool, text: String) -> Self {
		Self {
			text: Some(Cow::Owned(text)),
			..Self::cached(changed, "")
		}
	}

	pub(super) fn observe(mut self, observed: Option<u32>, this_run: u32) -> Self {
		self.observed = observed;
		self.this_run = this_run;
		self
	}

	/// Whether a queried component changed since this adapter last handled its row.
	///
	/// Includes application writes, while excluding confirmed writes made by this
	/// adapter. Use this for each relevant component alongside `is_changed()`.
	/// Ordinary Bevy change detection remains available for other policies.
	pub fn component_changed(&self, component: &impl DetectChanges) -> bool {
		let Some(observed) = self.observed else {
			return true;
		};

		component
			.last_changed()
			.is_newer_than(Tick::new(observed), Tick::new(self.this_run))
	}

	/// Whether the binding needs a value for a new catalog, Msg or query entry.
	///
	/// This does not include application components: inspect them through
	/// `component_changed`, native Ref/Mut ticks, or compare with `text()`.
	/// Use the context helper when your own deferred writes should be excluded.
	#[inline]
	pub fn is_changed(&self) -> bool {
		self.changed
	}

	/// Get this invocation's translation, reusing it on subsequent calls.
	#[inline]
	pub fn text(&mut self) -> &str {
		self.read = true;
		self.prepare_cache();

		if let Some(cache) = self.cache.as_deref() {
			return cache;
		}

		if self.text.is_none() {
			self.text = Some(Cow::Owned(self
				.render
				.expect("lazy localization formatter")()));
		}

		self.text
			.as_deref()
			.expect("localization value initialized")
	}

	/// Write a String field without marking an equal value as changed.
	///
	/// Pass a Bevy `Mut<String>` projection, for example
	/// `update.set_text(target.map_unchanged(|target| &mut target.0))`.
	/// This also reuses a cached buffer when a growing translation can be moved
	/// into the target. Subsequent `text()` calls still return the same value.
	/// Returns whether the field changed.
	pub fn set_text(&mut self, mut target: Mut<'_, String>) -> bool {
		self.read = true;
		let value = target.bypass_change_detection();

		if let (Some(cache), Some(fresh)) = (self.cache.as_deref_mut(), self.fresh.as_ref())
			&& value != fresh
			&& value.capacity() < fresh.len()
			&& cache.capacity() >= fresh.len()
		{
			fresh.clone_into(cache);
			*value = self.fresh.take().expect("fresh localization output");
			target.set_changed();
			return true;
		}

		let text = self.text();

		if value == text {
			return false;
		}

		text.clone_into(value);
		target.set_changed();
		true
	}

	pub(super) fn prepare_cache(&mut self) {
		if let Some(fresh) = self.fresh.take() {
			**self.cache.as_mut().expect("prepared cache") = fresh;
		}
	}

	pub(super) fn was_read(&self) -> bool {
		self.read
	}

	pub(super) fn into_owned(mut self) -> Option<String> {
		self.prepare_cache();

		if let Some(cache) = self.cache {
			return Some(cache.clone());
		}

		self.text.map(Cow::into_owned)
	}
}
