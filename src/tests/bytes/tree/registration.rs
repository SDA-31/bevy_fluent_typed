use super::{Hud, Panel, Root};
use crate::bevy::prelude::{App, MinimalPlugins};
use crate::{FluentScope, LocalizationPlugin};

#[test]
fn conflict_reports_the_complete_schema_despite_disjoint_source_leaves_and_locales() {
	let first = LocalizationPlugin::<Root>::from_bytes([(
		"en",
		Hud::module_paths()[0],
		b"HUD only".as_slice(),
	)])
	.unwrap();
	let second = LocalizationPlugin::<Root>::from_bytes([(
		"es",
		Panel::module_paths()[0],
		b"Panel only".as_slice(),
	)])
	.unwrap();
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, first));
	let error = second.validate_registration(&app).unwrap_err();
	assert_eq!(
		error.modules,
		[
			"other.ftl",
			"presentation/hud.ftl",
			"presentation/panel.ftl",
		]
	);
}
