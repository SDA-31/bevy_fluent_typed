## Full, Lazy and hybrid loading

`Localization<C, Full>` and `LocalizationPlugin<C, Full>` use Full by default.
Full requests every module of the selected locale. It exposes no load/unload API.
Lazy starts without demand:

```rust,ignore
use bevy_fluent_typed::{Lazy, Localization, LocalizationPlugin};
app.add_plugins(LocalizationPlugin::<texts::Translations, Lazy>::new(manifest));
// Equivalent initializer:
// LocalizationPlugin::<texts::Translations>::new_lazy(manifest)

fn enter(mut localization: ResMut<Localization<texts::Translations, Lazy>>) {
    localization.load::<texts::Presentation>();
    localization.load::<texts::presentation::Hud>();
}

fn leave(mut localization: ResMut<Localization<texts::Translations, Lazy>>) {
    localization.unload::<texts::Presentation>(); // Hud's independent request remains.
}
```

Requests are idempotent per scope type. Root requests need every leaf; group
requests need descendants. Overlap is the union of outstanding requests. Hybrid
loading means retaining a base scope request and loading other scopes as needed.
A repeated failed `load` retries; accessors themselves never initiate loading.
`Full` and `Lazy` implement the sealed `LoadingMode` trait.

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn request<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    state.load::<C>();
}
```

```compile_fail,E0599
use bevy_fluent_typed::{FluentCatalog, Full, Localization};
fn release<C: FluentCatalog>(state: &mut Localization<C, Full>) {
    state.unload::<C>();
}
```

```compile_fail
use bevy_fluent_typed::{FluentCatalog, FluentScope, Lazy, Localization};
fn wrong_root<C: FluentCatalog, S: FluentScope>(state: &mut Localization<C, Lazy>) {
    state.load::<S>(); // S::Catalog must be C.
}
```

```compile_fail
use bevy_fluent_typed::LoadingMode;
struct Other;
impl LoadingMode for Other { const FULL: bool = false; }
```

## Resources, navigation and bindings

Complete root/group/leaf scopes are published as shared immutable resources.
A leaf can exist while its parent/root is incomplete. `localization.catalog()`
returns `Option<&C>`. Schema navigation remains available while unloaded:

```rust,ignore
let hud = localization.modules().presentation().hud()?;
// `hud` borrows the controller's store, not a temporary view.
let title = hud.msg_title();
let message = Message::new(|hud: &texts::presentation::Hud| hud.msg_title());
let binding = LocalizedText::<texts::presentation::Hud>::new(|hud| hud.msg_title());
```

Group navigation returns another view. Leaf access returns `Result<&Leaf,
ModuleError>` with locale, logical path and latest loading status. `status::<S>()`
reports Unloaded/Loading/Ready/Failed. A failed reload can still have an available
previous valid same-language value; availability and last-attempt status differ.
`Message<S>`/`LocalizedText<S>` support leaf, group and root with no mode generic.
Bindings update existing Text/Text2d entities, clearing text when their scope is
unavailable and refreshing when inserted. They never request modules. Keep
editable drafts separate and replace bindings when captured arguments change.

## Locale changes, failures and ownership

`set_locale` keeps requests but clears previous-language state and handles.
No automatic fallback or language cache exists. Startup follows the manifest's
known default language unless a controller was inserted before the plugin.
A mismatching source language or unknown default is rejected explicitly.
Only one plugin mode may own a given root provider in an App.

Successful checked leaves publish independently. A same-language invalid reload
retains that leaf's last good value; valid siblings can still update. Switching
languages cannot reuse that old value as fallback. Every successful reload
produces a fresh snapshot, including identical text. Idle frames and unchanged
siblings keep resource identity and Bevy change ticks.

`CatalogUpdate::Loaded { locale, path }` and `Rejected { locale, path, error }`
identify individual module attempts. Observe via `CatalogUpdateReader` after
`LocalizationSystems::Publish`. Send `ReloadCatalogs` to retry demanded FTL,
including initially missing files. It does not reread TOML: the supplied manifest
is an immutable contract. Bevy 0.16 uses EventWriter/send_event; newer backends
use MessageWriter/write_message. A read-but-invalid file remains watched and may
recover automatically when `watch` and its source support watching.

Localization-owned reloads are serialized per module, including requests queued
while the initial read is pending; repeated pending retries coalesce. Loader
revisions also prevent an earlier loader from replacing a later accepted result
when their `read_to_end` futures complete in reverse order.

Automatic watcher reloads and direct application `AssetServer::reload` calls are
outside that queue. Bevy opens an `AssetReader` before invoking the loader and
exposes no request generation at that boundary. If an older reader-opening
future completes after a newer one, chronology cannot be recovered by this
integration. `AssetLoadFailedEvent` likewise carries no request generation,
including failures inside the loader's `read_to_end`. Strict ordering therefore
requires disabling automatic watching and routing reloads through `ReloadCatalogs`;
custom sources must also
provide coherent data. This crate does not wrap the application's reader or
perform a second I/O pass to infer freshness.

Publication occurs in PreUpdate's Publish and again before PostUpdate's Refresh.
Even embedded parsing starts during an update, not plugin initialization. Update
language changes reach resources in PostUpdate; order consumers after Refresh
for synchronized text. The controller itself changes immediately, so earlier
same-frame direct resources can still represent the preceding publication.

Unloading the final request drops runtime strong handles, leaves and ancestor
snapshots at synchronization; Bevy may finish retiring assets on later updates.
Caller-owned clones can intentionally retain parsed resources. Embedded static
source bytes cannot be freed. File-backed Lazy mode reads and retains only demand;
choose reasonably sized FTL leaves when a language contains gigabytes of text.
No hidden source String copy is retained just to compare reloads.

## Migration from registry 0.1.3

Pass a `LocalizationManifest` instead of a path string to the plugin. Choose an
explicit source, and replace implicit embedded startup assumptions with resource
availability checks. `Locale::load`, `Translations::embedded`, provider
`descriptor` and whole-language `parse` hooks are removed. Core accessors and
Arc-backed scope types remain; direct bytes use `Leaf::new` or root
`Translations::from_modules`, both checked by default. Safe `_unchecked` core
constructors skip schema validation only, still checking UTF-8 and Fluent syntax.
Separate validation methods permit checking data before loading it.

Handwritten providers implement `FluentScope` and `FluentCatalog`, declaring
checked `Module::new::<Leaf>` parsers and `ScopeRegistration::new::<Scope>`
assembly hooks. Assembly shares ready children and must not reread or reparse.
See the no_codegen example for the smallest provider. Numeric formatting,
fonts, shaping, storage transports and error presentation remain application policy.
