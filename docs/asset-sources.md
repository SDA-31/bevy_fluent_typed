## Custom asset sources

The manifest is a prepared contract, not a catalog or live TOML asset. Register
an application's `AssetReader` before `AssetPlugin`, then preserve its named source
in the origin:

```rust,ignore
use bevy_fluent_typed::{LocalizationManifest, LocalizationPlugin};

let manifest = LocalizationManifest::parse(
    texts::CATALOG_CONFIG,
    "translations://localizations/localization.toml",
)?;
app.add_plugins(LocalizationPlugin::<texts::Translations>::new(manifest));
```

For directory `translations`, a request for the quickstart's English
`presentation/hud.ftl` reads exactly
`translations://localizations/translations/en/presentation/hud.ftl` through AssetServer.
The TOML need not exist in that source: parsing has already supplied the contract.
Runtime directory/origin may differ from build inputs. Source/default language
metadata must be compatible with the compiled schema. Paths are validated and
mapped exactly; nested logical paths are never inferred from suffixes.

The source owns transport, decompression, caching and change notifications.
Archive support requires an appropriate reader; a source name is not a built-in
format selector. Explicit ReloadCatalogs works without watching and only rereads
demanded FTL. Keep a coherent source revision while loading several files:
schema validation cannot recognize mixed but otherwise valid prose revisions.
There is no whole-language transaction or implicit embedded rollback.

The [memory-source example](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/asset_source)
provides virtual files and tests missing data, rejected edits and recovery.
It explicitly embeds sample bytes only to seed its demonstration reader.
Downloading, signatures, installations and persistent rollback belong to the app.

The localization retry queue serializes its own requests, but cannot order
independent watcher/direct AssetServer reloads whose reader-opening futures finish
out of order. Bevy supplies no pre-loader request generation. Its
`AssetLoadFailedEvent` also has no generation, including failures inside
`read_to_end`. For strict order, disable watching and use ReloadCatalogs. See the
[loading boundary](https://github.com/SDA-31/bevy_fluent_typed/blob/main/docs/runtime.md#scheduling-and-reload-guarantees).
