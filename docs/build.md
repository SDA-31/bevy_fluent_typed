## Explicit build-script API

The `build` feature exposes [`build()`], [`from_cargo()`], [`generate()`] and
[`Settings`] through the same crate used by the application. The companion bridge
is an implementation detail; consumers do not need to name it in Cargo.toml.
Discovery and the typed module tree come from
[fluent_typed_codegen](https://docs.rs/fluent_typed_codegen/); message accessors
and Fluent resolution come from [fluent-typed](https://docs.rs/fluent-typed/).

In build-dependencies, disable default features and enable **only `build`**.
This configuration compiles the generator, not Bevy or the localization runtime.
Normal dependencies enable a Bevy backend and `codegen`, not `build`. Use Cargo
resolver 2 or 3 so host and target features remain separate.

```no_run
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

Generation and Cargo file/directory tracking run exclusively in this explicit
call. The application's `translations!` macro only includes the prepared output
under `OUT_DIR`; macro expansion never runs the generator or writes files.

For custom diagnostics use `from_cargo()` and handle its error; for explicit
package/output paths use `generate()` with `Settings`. Do not ignore generation
failures or compile stale output after an error.

Cargo feature forwarding such as `bevy_fluent_typed/bevy-0-17` can enable a feature
on both dependency kinds when they share a name. Select the backend directly in
the **normal dependency's** `features` array, not through application feature
forwarding. Cargo also requires the same dependency name in both sections; if
renaming the crate, use that alias in both, as in the integration example.

The public facade is available since **0.1.1**. Version 0.1.0 required a separate
bridge build-dependency. Number formatting and plural-category preparation are
application runtime work, not part of this build phase.
