# Custom asset source

Run from a checkout of this repository:

```sh
cargo run --manifest-path examples/asset_source/Cargo.toml
cargo test --manifest-path examples/asset_source/Cargo.toml
```

This headless example targets the default Bevy 0.19 backend. It uses Bevy's
`MemoryAssetReader` to demonstrate the source boundary without implementing an
archive format or adding archive dependencies. It is not a ZIP reader.
Build-time FTL files still define the generated API and embedded fallback.

[Source registration](src/application.rs) happens **before** `AssetPlugin`:

```rust
app.register_asset_source(
    "translations",
    AssetSourceBuilder::new(move || {
        Box::new(MemoryAssetReader { root: files.clone() })
    }),
);
```

After adding `AssetPlugin`, the ordinary localization plugin receives
`"translations://localizations/localization.toml"`. Both that definition and its
dependent FTL modules are read through the named source, not the default asset
folder. Replace the reader factory with an archive plugin's reader to use other
storage; the localization API and generated resources stay unchanged.

[main.rs](src/main.rs) waits for external loading, prints a typed HUD message,
changes a virtual file, sends `ReloadCatalogs` and waits for the new messages.
It then changes language. Output:

```text
Ready
Ready to explore
Listo
```

The in-memory source starts from generated `MODULES` solely to keep the example
self-contained; real external packs supply their own bytes. No watcher is used.
Updates happen between completed loads, not concurrently with reads.
`Outcomes` and the bounded polling loop are headless demonstration plumbing,
not extra application resources required by the localization plugin.

[Tests](src/tests.rs) check real generated root/module resources, `Text`/`Text2d`,
unchanged reloads, switching languages, per-language rejection, missing files,
invalid UTF-8, malformed definitions and recovery through explicit reloads.
There is no GPU/window test or archive parser in this package.

For pack layout, safe installation and watcher responsibilities, see
[custom asset sources](../../docs/asset-sources.md). Older backends have the same
localization contract, but Bevy 0.16/0.17 construct reader factories with
`AssetSourceBuilder::default().with_reader(...)`; Bevy 0.16 sends reloads with
`World::send_event`, not `World::write_message`.

Local path dependencies test this checkout. Public dependency installation is
documented in the main [README](../../README.md#getting-started).
[MIT](LICENSE).
