use crate::tests::TestCatalog;
use crate::{Localization, Manual};
use std::sync::Arc;

#[test]
fn selected_locale_preserves_active_data_and_logical_requests_until_ready() {
	let mut state = Localization::<TestCatalog, Manual>::default();
	assert_eq!(state.locale(), "ja");
	assert!(state.catalog().is_none());
	state.load::<TestCatalog>();
	state
		.store
		.insert_leaf("ui.ftl", Arc::new(TestCatalog("ready".into())));
	assert_eq!(state.catalog().unwrap().0, "ready");
	state.set_locale("es");
	assert_eq!(state.catalog().unwrap().0, "ready");
	assert_eq!(state.locale(), "ja");
	assert_eq!(state.prepared_locale(), Some("es"));
	assert!(state.desired().contains("ui.ftl"));
	state.unload::<TestCatalog>();
	assert!(state.desired().is_empty());
}
