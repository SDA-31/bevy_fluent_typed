## Custom asset sources and translation packs

Localization can use any registered Bevy `AssetReader`, not just loose files.
ZIP, SFS or another indexed archive needs an appropriate reader supplied by the
application or an asset plugin. No archive dependency, format-specific feature
or additional localization resource is required by `bevy_fluent_typed`.

Register the named source **before** adding `AssetPlugin` or `DefaultPlugins`.
Then pass its virtual definition address to the ordinary localization plugin:

```ignore
// `texts` is the application's generated catalog; the `translations` source
// has already been registered with Bevy.
app.add_plugins(LocalizationPlugin::<texts::Translations>::new(
    "translations://localizations/localization.toml",
));
```

The [runnable custom-source example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/asset_source)
shows source registration and generated resources in `main.rs`, using Bevy's
built-in memory reader. Its `tests/` directory covers language switching,
explicit reloads and text bindings. It requires no archive implementation.
The source name is an application-chosen identifier, not a URL protocol or a
format selector built into this crate. Merely naming it `zip` does not read ZIPs.

### Layout and build-time inputs

For `translations-directory = "translations"`, the source exposes paths such as:

```text
localizations/localization.toml
localizations/translations/en/ui/hud.ftl
localizations/translations/en/ui/panel.ftl
localizations/translations/es/ui/hud.ftl
localizations/translations/es/ui/panel.ftl
```

Every dependent FTL retains the definition's named source. Paths resolve beneath
the definition's parent, then the configured translations directory and locale.
They do not include the physical `assets/` folder or a source URL prefix inside
the archive. An archive may contain other assets; localization reads only its
definition and the provider's declared modules.

Code generation still reads ordinary source files during the explicit `build.rs`
step. Ship the original TOML and per-language FTL layout as runtime assets;
generated Rust or upstream intermediate bundle files are not a translation pack.
The build-time `asset-root` does not force the same physical runtime location.
Pack all declared languages/modules for a complete external replacement.
Compatibility validation is unchanged: prose edits need no new executable,
but changes to keys, argument contracts, references, languages, module inventory
or configuration require regeneration and rebuilding.

### Reload and publication

After installing compatible bytes in the source, send
`ReloadCatalogs::<Translations>::default()`. No `watch` feature is needed for an
explicit reload. With Bevy 0.17–0.19, use `MessageWriter` or `World::write_message`;
on 0.16 use `EventWriter` or `World::send_event`.

Loading is asynchronous. Observe `CatalogUpdate` after `LocalizationSystems::Publish`:
`Loaded` is reported per language; `Rejected { locale: None, .. }` means the
definition/aggregate failed. A language-specific rejection retains that language's
last good snapshot while other valid languages may update. Existing generated
`Res<...>` catalogs and `LocalizedText` bindings refresh through the usual systems.
Consumers ordered after `LocalizationSystems::Refresh` see updated text as well.

The source owns I/O, decompression, caching and notifications. The crate's `watch`
feature does not automatically watch an archive or map archive changes to virtual
entry events. Provide that through the source, or request reloads explicitly.

Keep one coherent source revision available throughout each multi-file load.
Replacing an archive atomically on disk does not by itself prevent consecutive
reads from seeing different revisions. Use immutable/versioned storage or
coordinate installation with loading. Fluent validation rejects incompatible
schemas, but cannot recognize mixed, otherwise valid prose revisions.

Embedded catalogs remain available at startup and on initial external failure.
The last good external snapshot is kept in memory, not persisted as an installer
rollback. This integration does **not** remove embedded translations from the
executable. Downloading, signatures, decompression limits and durable rollback
belong to the application/source, not the localization runtime.
