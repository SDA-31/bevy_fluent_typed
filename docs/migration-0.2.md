# Migrate from bevy_fluent_typed 0.1.3 to 0.2.1

This guide upgrades runtime and bridge 0.1.3 to 0.2.1.
It includes the generated manifest helper and native required-resource waiting.

Start with **Full**, the default mode. It keeps all modules of the selected
language requested, so you can retain existing full-tree message closures.
Unlike 0.1.3, resources are not ready immediately when the plugin is added.
Adopt **Lazy** separately if you want explicit module lifetimes.

## 1. Update the public facade

Replace the localization entries in your Cargo.toml:

```toml
[dependencies]
bevy_fluent_typed = { version = "0.2.1", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.2.1", default-features = false, features = ["build"] }
```

Keep your existing Bevy dependency and localization metadata. This example uses
the default Bevy 0.19 backend. For 0.16, 0.17 or 0.18, retain your matching backend
feature and `default-features = false` on the normal dependency only. Rust 1.95
and the supported Bevy families are unchanged. Use Cargo resolver 2 or 3.

Remove development Git/path patches for the runtime, bridge and generator when
switching to the registry. Standard consumers do not declare a bridge or generator
dependency; the facade resolves the matching 0.2.1 packages. If you rename the
facade, use the same alias in both dependency sections.

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
LocalizationPlugin::<texts::Translations>::new(texts::CATALOG_ASSET_PATH)
```

After, use the generated file-source contract:

```rust
let manifest = texts::manifest();
let plugin = bevy_fluent_typed::LocalizationPlugin::<texts::Translations>::new(manifest);
```

The helper uses the build-configured TOML and its **Bevy asset address**, without
reading files or embedding FTL. Keep `AssetPlugin` pointed at your deployment's
asset root. To change the runtime origin, construct a `LocalizationManifest`
explicitly; see [custom sources](asset-sources.md).

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

Existing root-bound closures still work in Full mode:

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

Full is enough for the minimal migration. To request only selected modules,
replace plugin construction with:

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
`Startup`. Reading a resource, navigation or a message does not request a module.
Requests for a type are idempotent, not reference-counted: one unload releases
all repeated loads of that same type. Independent parent/child requests can still
keep a leaf loaded. Full has no `load` or `unload` methods.

For selective embedding, use
`texts::embed_manifest!(module = texts::presentation::Hud)` and request that same
leaf. Embedded static bytes live for the executable's lifetime; unloading releases
runtime ownership of parsed data, not those bytes. Application-held clones can
also keep parsed scopes alive. See the [complete Lazy application](../GUIDE.md#fully-lazy-complete-mainrs).

## 6. Account for changed reload behavior

| In 0.1.3 | In 0.2.1 |
| --- | --- |
| All embedded languages were initialized | Full requests only the selected language; Lazy requests selected scopes |
| Locale changes could select a retained catalog immediately | Logical requests persist, old-language data is released and the new language loads |
| A complete language was checked and published together | Checked leaves publish independently; a bad same-language reload retains only that leaf's last good value |
| Unavailable bindings retained their old displayed text | Unavailable bindings clear until their scope is available |
| Unchanged sources could preserve the snapshot | Successful reloads publish fresh snapshots; idle frames and unchanged siblings retain identity |

Do not rely on an all-files transaction or automatic fallback. Keep source files
coherent when distributing a translation pack. `CatalogUpdate` identifies the
locale and logical leaf path; update exhaustive event matches for the new shape.
After a failed same-language reload, `ModuleStatus::Failed` can coexist with a
usable last-good resource. Check availability and latest-attempt status separately.

## 7. Handwritten providers and direct bridge users

Skip this section when using `translations!`; the bridge generates these changes.

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

Direct bridge users must upgrade `bevy_fluent_codegen_bridge` and the generator
to 0.2.1 together with the runtime. The old bridge `validate_definition` helper is
removed; use the shared manifest contract. Standard consumers keep the public
facade and need no direct bridge dependency.

Run `cargo check` and application tests after migration. Exercise startup,
language changes, invalid-file recovery and any screen unload/reload behavior.

[All changes](../CHANGELOG.md) · [Quickstart](../README.md#setup) · [Loading guide](../GUIDE.md)
