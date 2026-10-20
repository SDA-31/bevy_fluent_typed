//! Fixed groups must not block publication of unrelated following scopes.
use super::{Hud, Panel, Presentation, Root, controlled, pump};
use crate::{Localization, Manual, Msg};

#[test]
fn pending_fixed_sibling_does_not_block_switching_the_following_scope() {
	let (mut app, source) = controlled();
	{
		let mut state = app.world_mut().resource_mut::<Localization<Root, Manual>>();
		state.load::<Hud>();
		state.for_locale_mut("es").load::<Panel>();
	}
	pump(&mut app, |_| source.count() == 2);
	source.release::<Hud>("en", 0);
	pump(&mut app, |world| world.contains_resource::<Hud>());
	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.set_locale("es");
	pump(&mut app, |_| source.count() == 3);
	source.release::<Hud>("es", 0);
	pump(&mut app, |world| {
		world.resource::<Localization<Root, Manual>>().locale() == "es"
	});
	assert!(!app.world().contains_resource::<Panel>());
	source.release::<Panel>("es", 0);
	pump(&mut app, |world| {
		world
			.resource::<Localization<Root, Manual>>()
			.for_locale("es")
			.modules()
			.get::<Presentation>()
			.is_ok()
	});
	assert_eq!(source.count(), 3);
}

#[test]
fn fixed_group_and_leaf_share_reads_but_have_independent_release() {
	let (mut app, source) = controlled();
	let leaf = {
		let mut state = app.world_mut().resource_mut::<Localization<Root, Manual>>();
		state.for_locale_mut("es").load::<Presentation>();
		state.for_locale_mut("es").hold::<Hud>()
	};
	pump(&mut app, |_| source.count() == 2);
	let hud = source.release::<Hud>("es", 0);
	let panel = source.release::<Panel>("es", 0);
	pump(&mut app, |world| {
		world
			.resource::<Localization<Root, Manual>>()
			.for_locale("es")
			.modules()
			.get::<Presentation>()
			.is_ok()
	});
	let message =
		Msg::<Presentation>::new(|catalog| format!("{}|{}", catalog.hud.0, catalog.panel.0));
	let state = app.world().resource::<Localization<Root, Manual>>();
	let catalog = state
		.for_locale("es")
		.modules()
		.get::<Presentation>()
		.unwrap();
	assert_eq!(message.render(catalog), format!("{hud}|{panel}"));
	assert!(!app.world().contains_resource::<Presentation>());
	app.world_mut()
		.resource_mut::<Localization<Root, Manual>>()
		.for_locale_mut("es")
		.unload::<Presentation>();
	app.update();
	let state = app.world().resource::<Localization<Root, Manual>>();
	assert!(state.for_locale("es").modules().get::<Hud>().is_ok());
	assert!(state.for_locale("es").modules().get::<Panel>().is_err());
	drop(leaf);
	app.update();
	assert!(
		app.world()
			.resource::<Localization<Root, Manual>>()
			.for_locale("es")
			.modules()
			.get::<Hud>()
			.is_err()
	);
}
