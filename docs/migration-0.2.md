# Migrate from bevy_fluent_typed 0.1.3 to 0.3.0

This guide upgrades runtime 0.1.3 to 0.3.0.
It includes the generated manifest helper and native required-resource waiting.

Start with default **Auto**: inserted text bindings and localized systems request
their scopes automatically. Existing full-tree bindings request the whole tree;
prefer smaller scopes where appropriate. Resources become ready asynchronously.
Select explicit **Full** for eager loading or direct world polling, or **Lazy**
when the application must own requests manually.

## 1. Update the public facade

Replace the localization entries in your Cargo.toml:

```toml
[dependencies]
bevy_fluent_typed = { version = "0.3.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.3.0", default-features = false, features = ["build"] }
```

The dependency snippet selects Bevy 0.20 by default. Upgrade the application's
Bevy dependency to `"0.20"`, or retain Bevy 0.16, 0.17, 0.18 or 0.19 by selecting
its matching backend with `default-features = false` on the normal dependency.
See the [0.3 migration](migration-0.3.md) for both setups. Rust 1.97.1 is required.
Use Cargo resolver 2 or 3 and update localization metadata as shown in
[Catalog-only build configuration](#catalog-only-build-configuration).

Remove development Git/path patches for the runtime, bridge and generator when
switching to the registry. Standard consumers do not declare a bridge or generator
dependency; runtime 0.3.0 uses generator 0.2.2. If you rename the facade, use the
same alias in both dependency sections.

Keep your existing `build.rs`:

```rust
fn main() -> std::process::ExitCode {
    bevy_fluent_typed::build()
}
```

Keep `bevy_fluent_typed::translations!(mod texts);` in application source. The
build call generates the schema, and the macro includes it. Neither embeds FTL
unless you explicitly invoke the embedding macro.

## 2. Supply a source contract

Before, the plugin accepted a manifest asset address and started with embedded
catalogs while external files loaded:

```rust
LocalizationPlugin::<texts::Translations>::new("localizations/localization.toml")
```

After, use the generated file-source contract:

```rust
let manifest = texts::manifest();
let plugin = bevy_fluent_typed::LocalizationPlugin::<texts::Translations>::new(manifest);
```

The helper uses the build-configured TOML and its package-relative `CATALOG_PATH`,
without reading files or embedding FTL. Set the application's `AssetPlugin` root
so this origin resolves in the deployment layout. To change the runtime origin,
construct a `LocalizationManifest` explicitly; see [custom sources](asset-sources.md).

The plugin no longer loads or watches the TOML itself. To supply runtime TOML,
obtain its readable text before constructing this contract. `from_file` uses the
native filesystem; its filesystem origin does not automatically become a Bevy
asset address. See [custom sources](asset-sources.md).

For embedded data instead, replace the manifest initializer with:

```rust
let manifest = texts::embed_manifest!();
```

This includes all known languages' raw FTL and parses only the selected language.
Files and embedded data are explicit choices; failed file loads no longer fall
back to implicitly embedded text. Keep `AssetPlugin` installed for both choices.

## 3. Wait for resources, including with embedded data

Before, a system could read `Res<texts::presentation::Hud>` immediately.
Keep that parameter and register recurring consumers with the readiness helper:

```rust
use bevy_fluent_typed::LocalizationAppExt;

app.add_localized_systems(Update, show_title);
```

The system itself still uses Bevy's native resource:

```rust
fn show_title(hud: Res<texts::presentation::Hud>) {
    println!("{}", hud.msg_title());
}
```

It runs only while the HUD is available. This applies to startup, language
changes and explicit unloading, for both file and embedded sources.
A system with several required catalog parameters waits for all of them.

For one-time initialization after loading, register it separately:

```rust
app.add_localized_startup_systems(setup_hud);
```

This helper runs once in `Update`; ordinary `Startup` does not retry a skipped
system. For a complete application, follow the [quickstart](../README.md#setup).
The [resource-waiting reference](migration-0.2.1.md) covers scheduling constraints
and parameter shapes that require explicit readiness conditions.

A full-tree consumer can likewise request `Res<texts::Translations>` through the
helper. Controller access through `localization.catalog()` instead returns
`Option<&Translations>` because a complete tree may not be ready. Resource
readiness follows the [publication schedule](../GUIDE.md#scheduling-and-reload-guarantees).

## 4. Keep existing messages, or narrow their scope

Existing root-bound closures still work: in Auto an inserted root binding
requests the entire tree; explicit Full keeps it loaded independently of bindings:

```rust
use bevy_fluent_typed::Message;

let title = Message::<texts::Translations>::new(|texts| {
    texts.presentation().hud().msg_title()
});
```

A root binding waits until the entire root is available. To render as soon as
the HUD is ready, use its generated type:

```rust
use bevy_fluent_typed::Message;

let title = Message::<texts::presentation::Hud>::new(|hud| hud.msg_title());
```

Use `LocalizedText<texts::presentation::Hud>` for the corresponding binding.
Capture owned arguments in deferred closures; do not capture a translated string
or an old scope snapshot. Typed message accessor signatures remain unchanged.

## 5. Optionally adopt Lazy

Auto already requests only consumer scopes. Choose Lazy only to control requests
explicitly; its bindings and waiting systems do not create demand. Replace plugin
construction with:

```rust
LocalizationPlugin::<texts::Translations>::new_lazy(manifest)
```

The equivalent explicit type is
`LocalizationPlugin::<texts::Translations, Lazy>::new(manifest)`. Import `Lazy`
from `bevy_fluent_typed`, and update controller system parameters to the same mode:

```rust
use bevy_fluent_typed::{Lazy, Localization};

fn open_hud(mut localization: ResMut<Localization<texts::Translations, Lazy>>) {
    localization.load::<texts::presentation::Hud>();
}

fn close_hud(mut localization: ResMut<Localization<texts::Translations, Lazy>>) {
    localization.unload::<texts::presentation::Hud>();
}
```

Schedule these on screen entry and exit, or request an always-needed scope in
`Startup`. In this explicit Lazy mode, resource systems, bindings, navigation and messages
do not request modules automatically.
Requests for a type are idempotent, not reference-counted: one unload releases
all repeated loads of that same type. Independent parent/child requests can still
keep a leaf loaded. Full has no `load` or `unload` methods.

Lazy also works with the no-argument embedded manifest from step 2. It controls
which catalogs are parsed and retained; embedded static bytes remain for the
executable's lifetime. Application-held catalog clones can keep parsed scopes
alive after unloading. See the [complete Lazy application](../GUIDE.md#manual-lazy-loading).
For selective constant manifests, follow the
[embedded recipe](../README.md#explicit-embedding).

## 6. Account for changed reload behavior

| In 0.1.3 | In 0.3.0 |
| --- | --- |
| All embedded languages were initialized | Auto requests consumer scopes in the selected language; explicit Full requests that entire language |
| Locale changes could select a retained catalog immediately | Requests persist; the active language stays usable until all needed target leaves validate and switch automatically |
| A complete language was checked and published together | Same-language reloads publish leaves independently; locale replacement validates and publishes its desired target leaves together |
| Unavailable bindings retained their old displayed text | Unavailable bindings clear until their scope is available |
| Unchanged sources could preserve the snapshot | Successful reloads publish fresh snapshots; idle frames and unchanged siblings retain identity |

Same-language reloads have no all-files transaction or automatic fallback.
`set_locale` retains active resources while validating and automatically publishing
the desired target scopes; see [language switching](migration-0.3.md#let-language-changes-finish-automatically). Keep source files
coherent when distributing a translation pack. `CatalogUpdate` identifies the
locale and logical leaf path; update exhaustive event matches for the new shape.
After a failed same-language reload, `ModuleStatus::Failed` can coexist with a
usable last-good resource. Check availability and latest-attempt status separately.

## 7. Handwritten providers and direct bridge users

Skip this section when using `translations!`; the build facade generates these changes.

| Removed or changed contract | Replacement |
| --- | --- |
| `CatalogDescriptor` and `FluentCatalog::descriptor` | Construct `LocalizationManifest` separately |
| `FluentCatalog::embedded` and `Module::embedded` | Supply an explicit source contract |
| `ModuleSource` and whole-language `FluentCatalog::parse` | Register each leaf with `Module::new::<Leaf>(path, checked_parser)` |
| `FluentCatalog::modules(locale)` | `modules()` returns locale-independent checked parser registrations |
| `publish_resources` | Register root, groups and leaves through `ScopeRegistration::new::<Scope>()` |
| No scope trait/navigation contract | Implement `FluentScope` plus `FluentCatalog::Modules`, `scopes` and `view` |

Scopes implement `Resource + Clone`. Each scope lists its leaf paths and assembles
from ready children through `ModuleStore`; assembly must not reread or reparse
sources. Preserve contract validation in custom leaf parsers. Start from the
[complete handwritten provider](../examples/no_codegen/src/texts.rs), then register
all parent scopes before their children. Only one controller/plugin mode may own
a root provider in an App.

Direct bridge users should replace that dependency with `bevy_fluent_typed`:
use `build` with defaults disabled in build-dependencies, and `codegen` with a
backend in normal dependencies. The separate bridge package is no longer used.
The public `build()` and `translations!` entrypoints stay the same. For manual
providers using `default-features = false`, add `manifest` to retain file/manifest
constructors, or switch to [byte sources](../GUIDE.md#custom-byte-sources).

Run `cargo check` and application tests after migration. Exercise startup,
language changes, invalid-file recovery and any screen unload/reload behavior.

[All changes](../CHANGELOG.md) · [Quickstart](../README.md#setup) · [Loading guide](../GUIDE.md)

## Catalog-only build configuration

Merge the previous `asset-root` and
`catalog` values into one path relative to the package's `Cargo.toml`:

```toml
[package.metadata.localization]
catalog = "assets/localizations/localization.toml"
```

Remove `asset-root`; unknown fields are ignored and no longer affect path
resolution. Recognized fields are still validated. In explicit generator
settings, keep only `Settings { catalog: ... }`. Generated `CATALOG_PATH` replaces
`CATALOG_ASSET_PATH`, and `ASSET_ROOT` is removed. The catalog path may contain
`..` to share source translations across packages. Runtime logical module paths
and `translations-directory` still cannot escape their declared scope.

Use runtime 0.3.0 with generator 0.2.2. See the
[catalog path and embedding migration](migration-0.2.2.md) for the complete changes.

`texts::manifest()` retains `CATALOG_PATH` verbatim. Configure the application's
Bevy asset source root so that origin resolves, or supply a contract with the
chosen runtime origin. With the standard `assets` source root, use:

```rust,ignore
let manifest = bevy_fluent_typed::LocalizationManifest::parse(
    texts::CATALOG_CONFIG,
    "localizations/localization.toml",
)?;
```

Generation and explicit embedding do not depend on that engine address.
