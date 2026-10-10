# Verification

Run from the Bevy localization repository root using generator 0.2.2. For local
generator changes, configure the [override](../../../docs/build.md#work-on-local-checkouts).
For application setup, follow the
[quickstart](../../../README.md#setup).

```sh
cargo test --manifest-path examples/minimal/Cargo.toml
```

Application usage is documented in the [example README](../README.md). The tests
are separate Cargo integration targets; no test assertions run in the examples.

- [catalog.rs](catalog.rs): discovered languages, source metadata, embedded and
  external parsing, deferred messages and include-scope isolation.
- [catalog/contracts.rs](catalog/contracts.rs): strict key, argument and reference
  validation in both engine-free and Bevy-generated catalogs.
- [catalog/macros.rs](catalog/macros.rs): renamed dependencies, caller-name
  shadowing, nested/restricted visibility, forwarded macro attributes and embedded
  leaf/group/root constants. A private tree exports a source constant and catalog
  alias; a required `Res<Interface>` follows Manual loading, language changes,
  unloading and reloading.
- [catalog/lazy.rs](catalog/lazy.rs): independent scope demand, retained snapshots,
  idle change ticks and a typed leaf-only embedded source across locale changes.
- [catalog/resources.rs](catalog/resources.rs): shared root/folder/leaf snapshots,
  locale-switch scheduling, module inventory, local references and structured results.
- [catalog/reload.rs](catalog/reload.rs): real filesystem watching, rejection,
  recovery and per-frame resource/`Text`/`Text2d` consistency.
- [catalog/plurals.rs](catalog/plurals.rs): application-owned ICU formatting,
  deferred decimal/plural arguments, visible precision and native Fluent selectors.
- [raw_catalog.rs](raw_catalog.rs): engine-free generated output included at the
  crate root; that scope is intentionally distinct from the other targets.
- [typed_resources.rs](typed_resources.rs): executable output from the
  [typed-resource walkthrough](../src/bin/typed_resources.rs).

Watcher tests modify disposable copied assets, never the example sources.
They allow up to 15 seconds for convergence. Separate valid edits may publish
intermediate snapshots; they do not claim a multi-file filesystem transaction.

The eleven build-time modules include deliberate Rust keyword and standard-library
name collisions. `presentation/hud.ftl` and `presentation/panel.ftl` repeat local
keys with different argument contracts; `catalog.ftl` checks that a domain
`Catalog` coexists with the root `Translations`. Keep these fixtures when moving
or simplifying the application.

ICU is only a dev-dependency here. Tests preserve Fluent isolation but do not
render pixels or verify fonts, shaping or visual RTL layout. The
[ICU resource example](../../icu) adds Arabic digits and percentage formatting.
