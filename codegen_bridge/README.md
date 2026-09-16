# bevy_fluent_codegen_bridge

Connect `fluent_typed_codegen` to `bevy_fluent_typed`: generate a provider and
immutable Bevy resource types from modular Fluent files. The generator owns
discovery and the typed translation API; the runtime owns asset loading,
language switching, hot reload and text bindings.

This companion package lives in the Bevy integration repository under
`codegen_bridge/`. Its declared minimum Rust version is 1.95.

## Features

| Feature | Responsibility | Dependencies |
| --- | --- | --- |
| none (default) | No integration code | None |
| `build` | Generate the Bevy provider and resource declarations | `fluent_typed_codegen`; no Bevy |
| `runtime` | Macro expansion and immutable definition validation | Fluent syntax and TOML parsing; no generator or Bevy |

The main runtime's `codegen` feature enables this package's `runtime` feature.
Applications normally invoke `bevy_fluent_typed::translations!(pub mod texts)`.
They use this package directly only as a build-dependency:

```toml
[build-dependencies]
bevy_fluent_codegen_bridge = { git = "https://github.com/SDA-31/bevy_fluent_typed.git", branch = "main", features = ["build"] }

# Required in the consuming root until the generator is on crates.io.
[patch.crates-io]
fluent_typed_codegen = { git = "https://github.com/SDA-31/fluent_typed_codegen.git", branch = "main" }
```

Cargo finds this nested package in the Bevy repository by its package name.
Use the same Git revision for the runtime and bridge. Cargo.lock pins resolved
revisions; `rev` can replace `branch` when explicit pins are preferred.

```rust
fn main() -> std::process::ExitCode {
    bevy_fluent_codegen_bridge::build()
}
```

`from_cargo()` offers fallible error handling; `generate(package, output, settings)`
supports explicit build frontends. `Settings` is re-exported only with `build`.
The minimal consumer lives at [../examples/minimal](../examples/minimal/README.md).
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

`fluent_typed_codegen = "0.1.0"` is a versioned dependency, not a sibling path.
It explicitly enables the generator's `build` feature with defaults disabled;
the bridge's runtime-only feature still does not depend on the generator.
Git consumers supply the generator using the root `[patch.crates-io]` above;
local development can instead patch it to a checkout. Keep this override until
crates.io publication, even for runtime-only dependency resolution: Cargo may
resolve optional packages while building a lockfile without compiling them.
Patches in a dependency's manifest do not
propagate to consumers. Publish the generator first,
then this bridge, then the Bevy runtime. The bridge shares the runtime's repository.

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
