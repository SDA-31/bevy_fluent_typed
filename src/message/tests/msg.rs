use crate::bevy::prelude::Text;
use crate::tests::TestCatalog;
use crate::{Localized, Msg};

#[test]
#[allow(deprecated, reason = "Verify the public Message compatibility alias")]
fn deprecated_message_alias_preserves_constructors_and_bindings() {
	let legacy = crate::Message::<TestCatalog>::new(|catalog| catalog.0.clone());
	let current: Msg<TestCatalog> = legacy.clone();
	let legacy_again: crate::Message<TestCatalog> = current.clone();
	assert!(current.same_formatter(&legacy_again));
	assert_eq!(legacy.render(&TestCatalog("ja".into())), "ja");
	assert_eq!(current.render(&TestCatalog("es".into())), "es");

	let legacy_binding = Localized::<Text, _>::from(legacy);
	let current_binding = Localized::<Text, _>::from(current);
	assert!(
		legacy_binding
			.message
			.same_formatter(&current_binding.message)
	);

	let legacy_literal: Msg<TestCatalog> = crate::Message::literal("fixed");
	let current_literal: crate::Message<TestCatalog> = Msg::literal("fixed");
	assert_eq!(legacy_literal.render(&TestCatalog("ja".into())), "fixed");
	assert_eq!(current_literal.render(&TestCatalog("es".into())), "fixed");
}
