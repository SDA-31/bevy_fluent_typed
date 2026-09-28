# bevy_fluent_codegen_bridge

Companion for runtime 0.2.x. Follow the [runtime setup](../README.md#setup)
for installation. The [migration guide](../docs/migration-0.2.md#7-handwritten-providers-and-direct-bridge-users)
covers provider changes; bridge release notes live in the shared
[changelog](../CHANGELOG.md).

This optional bridge emits Bevy resource declarations, checked per-leaf parsers,
ready-child assembly and typed navigation. The generator owns discovery, schemas
and accessors; runtime owns I/O, demand, publication and text bindings. Fluent
resolution uses fluent-typed. Rust minimum is 1.95.

| Feature | Responsibility |
| --- | --- |
| none (default) | No code or dependencies |
| build | Explicit generation using fluent_typed_codegen/build; no Bevy |
| runtime | Output include macro and Fluent syntax support; no generation code |

Standard consumers use bevy_fluent_typed in both dependency sections, selecting
`build` only on the default-disabled host dependency and `codegen` on the runtime.
The bridge never depends back on the runtime. Its macro receives the runtime
path; generated code gets shared manifest support through that runtime alias.
Resolver 2/3 keeps features separate. No generation occurs in macro expansion.

Declaring the generated module embeds no FTL, even in unoptimized builds.
`texts::manifest()` returns the validated build-configured file-source contract
for `LocalizationPlugin::new`; it performs no I/O and embeds no translation text.
Bevy still owns the asset root. Supply a different `LocalizationManifest` when
runtime paths or storage differ from the build configuration.
Explicit `texts::embed_manifest!()` includes every leaf; a selector such as
`module = texts::presentation::Hud` or `module = texts::Presentation` includes
only that leaf or group's descendants, across known languages. See
[embedding](../README.md#explicit-embedding) for setup and examples.

Low-level `build()`, `from_cargo()`, `generate()` and `Settings` remain available with
`build`; prefer the runtime facade. There is no sibling-path generator dependency.
The required generator version is 0.2.0. For source development, use the
[local generator patch](../docs/build.md#work-on-local-checkouts).

Generated scope views live in a hidden namespace to avoid public message/type
collisions. Leaves borrow the underlying store lifetime, so chained temporary
views do not shorten the result. Resource declarations use the runtime macro:
Bevy0.19 enforces immutable resources; older supported versions use read-only APIs.
Manifest metadata validation belongs to the shared contract/runtime; the old
bridge validate_definition helper and TOML dependency are removed.

Implementation: generation.rs coordinates typed extension hooks; provider.rs
emits root descriptors; scopes.rs emits ready-child assembly; navigation.rs emits
schema views; macros.rs includes prepared output. syn/quote are build-only.

Application-owned ICU/ICU4X formatting supplies ordinary String arguments and
plural keywords. Font coverage, RTL layout and shaping remain renderer concerns.
See [number formatting](../docs/formatting.md). MIT: [LICENSE](LICENSE).
