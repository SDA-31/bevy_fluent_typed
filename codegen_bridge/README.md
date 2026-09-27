# bevy_fluent_codegen_bridge

Unreleased companion on `feat/runtime-module-loading`; registry 0.1.3 retains the
previous provider API. Follow the [runtime Git setup](../README.md#development-setup),
including the pinned generator patch. Package versions have not changed.

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

Low-level `build()`, `from_cargo()`, `generate()` and `Settings` remain available with
`build`; prefer the runtime facade. There is no sibling-path generator dependency.
The declared registry requirement remains 0.1.4 while this development branch
requires the README's explicit patch to the approved generator revision.

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
