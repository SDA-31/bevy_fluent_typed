# Bevy 0.20 compatibility preview

This is a **Git-only experiment**, not a crates.io release or a stable-support
promise. The branch `compat/bevy-0.20` tracks one explicitly checked candidate:
Bevy **0.20.0-rc.1**, requiring Rust **1.96 or newer**. Default features still
select stable Bevy 0.19; use `bevy-0-20` explicitly with defaults disabled.
The branch includes the 0.2.1 loading API; publication is disabled.

## Try it in an application

Use the same Git source in both dependency sections:

```toml
[dependencies]
bevy_fluent_typed = { git = "https://github.com/SDA-31/bevy_fluent_typed", branch = "compat/bevy-0.20", default-features = false, features = ["bevy-0-20", "codegen", "watch"] }

[build-dependencies]
bevy_fluent_typed = { git = "https://github.com/SDA-31/bevy_fluent_typed", branch = "compat/bevy-0.20", default-features = false, features = ["build"] }

[package.metadata.localization]
asset-root = "assets"
catalog = "localizations/localization.toml"
```

Keep the explicit build.rs call and `bevy_fluent_typed::translations!(mod texts)`
from the normal quick start. The build dependency must not enable an engine
backend. The build adapter is part of `bevy_fluent_typed` and uses the published
`fluent_typed_codegen` 0.2.1. No bridge dependency or generator patch is needed.

If the application also depends directly on Bevy, select exactly
`version = "=0.20.0-rc.1"` with the application's own engine features. Do not
combine this backend with a stable Bevy version. For reproducible installation,
replace `branch` with the same reviewed commit `rev` in both entries; a branch
name can move. The lockfile records the chosen Git revision.

## Verification and lifetime

CI keeps the stable compatibility matrix and adds Linux checks of this exact RC
on current stable Rust and Rust 1.96.0. RC-specific Windows/macOS compatibility is
not promised by this first preview. Checks cover typed codegen, existing UI/world
text on locale changes, hot reload, last-good recovery, ICU consumer formatting,
named asset sources, Full/Lazy module lifetimes, native resource readiness,
byte sources, host/runtime feature isolation and immutable resources.
The quality job also checks the RC backend with Clippy and Rustdoc.

Run the same integration checks locally from this repository:

```sh
cargo run --manifest-path tools/compatibility/Cargo.toml -- \
  --generator /absolute/path/to/fluent_typed_codegen \
  --target-dir /absolute/path/to/build-cache \
  0.20.0-rc.1
```

The generator checkout must match the revision pinned by `.github/workflows/ci.yml`.
The tool rejects unreviewed RCs, validates the exact engine release family and
keeps its temporary fixtures and diagnostics. It does not alter application files.

The annotated Git-only tag `bevy-0.20.0-rc.1` identifies a verified preview for
this Bevy candidate. It is updated after merging the 0.2.1 API from `main` and
passing CI. Pin a reviewed commit `rev` to keep a fixed source revision; this
preview tag can move. Stable crate release tags remain fixed. Do not publish
RC packages from this branch to crates.io.

When stable Bevy 0.20 arrives, review the final dependency/API changes and run CI
again before merging support into `main`. Publish only the subsequent stable
crate release. The compatibility branch can then be deleted; its tags remain.
