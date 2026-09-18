# bevy_fluent_codegen_bridge

[![crates.io](https://img.shields.io/crates/v/bevy_fluent_codegen_bridge)](https://crates.io/crates/bevy_fluent_codegen_bridge)
[![docs.rs](https://img.shields.io/docsrs/bevy_fluent_codegen_bridge)](https://docs.rs/bevy_fluent_codegen_bridge/latest/bevy_fluent_codegen_bridge/)
[![CI](https://img.shields.io/github/actions/workflow/status/SDA-31/bevy_fluent_typed/ci.yml?branch=main&label=CI&logo=github)](https://github.com/SDA-31/bevy_fluent_typed/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/crates/msrv/bevy_fluent_codegen_bridge)](https://crates.io/crates/bevy_fluent_codegen_bridge)
[![License](https://img.shields.io/crates/l/bevy_fluent_codegen_bridge)](LICENSE)

Connect [fluent_typed_codegen](https://github.com/SDA-31/fluent_typed_codegen) to
[bevy_fluent_typed](https://github.com/SDA-31/bevy_fluent_typed): generate a provider and
shared Bevy resource types from modular Fluent files. The generator owns
discovery and the typed translation API; the runtime owns asset loading,
language switching, hot reload and text bindings.
The actual message accessors and Fluent resolution are supplied by
[fluent-typed](https://github.com/human-solutions/fluent-typed).

This companion package lives in the Bevy integration repository under
`codegen_bridge/`. Its declared minimum Rust version is 1.95.

[API documentation](https://docs.rs/bevy_fluent_codegen_bridge/latest/bevy_fluent_codegen_bridge/) ·
[Integration guide](../GUIDE.md)

Engine selection belongs only to the runtime. The bridge emits a runtime-owned
declaration macro: Bevy 0.19 receives ECS-immutable resources, while older backends use
ordinary resources around the same read-only snapshots. Do not add backend flags
to build-dependencies; host and runtime Cargo feature graphs are separate.

## Features

| Feature | Responsibility | Dependencies |
| --- | --- | --- |
| none (default) | No integration code | None |
| `build` | Generate the Bevy provider and resource declarations | `fluent_typed_codegen`; no Bevy |
| `runtime` | Macro expansion and immutable definition validation | Fluent syntax and TOML parsing; no generator or Bevy |

The main runtime's `codegen` feature enables this package's `runtime` feature.
Applications normally invoke `bevy_fluent_typed::translations!(pub mod texts)`.
Since 0.1.1 the facade also exposes generation as `bevy_fluent_typed::build()`.
Standard consumers use that crate in both dependency sections, enabling only
`build` with defaults disabled in build-dependencies. They do not name this bridge.
See the [facade setup](../README.md#optional-generation).

Direct bridge access remains available for low-level users:

```toml
[build-dependencies]
bevy_fluent_codegen_bridge = { version = "0.1.2", features = ["build"] }
```

The bridge and generator resolve from crates.io; no checkout or registry patch
is required. Cargo.lock pins the resolved versions.

```rust
fn main() -> std::process::ExitCode {
    bevy_fluent_codegen_bridge::build()
}
```

`from_cargo()` offers fallible error handling; `generate(package, output, settings)`
supports explicit build frontends. `Settings` is re-exported only with `build`.
The minimal consumer lives at [../examples/codegen](../examples/codegen/README.md).
Its resource path is `assets/localizations/translations/<locale>/`.
The TOML's optional `translations-directory` is relative to the definition file
and defaults to `"."`. The legacy `languages-directory` alias is also accepted;
providing both keys is an error. Definition reload compares resolved values, so
omission and explicit `"."` are equivalent, but changing the path requires regeneration.

## Dependency direction

The Bevy runtime optionally depends on this bridge. The bridge never imports the
runtime crate as a dependency: its macro receives the facade's runtime path,
and build-time code emits Rust source referring to that path. This avoids a
dependency cycle and keeps the generator independent of Bevy.

With Cargo resolver 2/3, normal use enables only `runtime`, while build-script
use enables `build` separately. An explicit `--all-features` build of this package
naturally includes both. Check separate consumer graphs when verifying isolation.

`fluent_typed_codegen = "0.1.3"` is a versioned dependency, not a sibling path.
This minimum includes explicit source-file tracking so deleting FTL modules also
regenerates the API on Windows. Version 0.1.3 also documents application-owned
ICU formatting without the retired Decimal adapter. Runtime and bridge package
versions are 0.1.2.
It explicitly enables the generator's `build` feature with defaults disabled;
the bridge's runtime-only feature still does not depend on the generator.
Local generator development can use a caller-owned `[patch.crates-io]` pointing
to its checkout. Patches in a dependency's manifest do not propagate to consumers.
For manual releases, publish the generator first,
then this bridge, then the Bevy runtime. The bridge shares the runtime's repository.

## Number and presentation boundaries

The bridge preserves upstream native numeric arguments and String selectors.
Applications should format numbers, percentages and currencies with dedicated
[ICU4X](https://docs.rs/icu/) or
[ICU](https://unicode-org.github.io/icu/userguide/format_parse/) components, then
pass formatted text and any plural keywords as ordinary String arguments.
Neither the bridge nor the generator depends on ICU. Fluent resolves the selected message, and the application
keeps numeric formatting aligned with the active catalog. The bridge does not
format numbers, watch files, shape Arabic text or implement visual RTL layout.
See the [plural guide](../GUIDE.md#decimal-and-plural-arguments).

## Implementation map

- `src/generation.rs`: implements the generator's neutral, typed `Extension`
  hooks using `syn::parse_quote!`; no escaped source strings. Scope paths and
  accessor identifiers are interpolated as syntax when publishing resources.
- `src/provider.rs`: constructs the generated `FluentCatalog` implementation.
- `src/macros.rs`: owns the generated output include and dependency aliases.
- `src/definition.rs`: compares parsed TOML against generated immutable fields.
- The generator emits its own strict Fluent validator into the checked API.
- The Bevy runtime owns loading, scheduling and publication, not this bridge.

The low-level macro also accepts an explicit runtime:
`translations!(runtime = my_runtime; pub mod texts)`. Prefer the facade unless
implementing custom wiring. The generated adapter, runtime trait and bridge
must be released and tested as compatible versions.

## License

[MIT](LICENSE), independently of any consuming application.
