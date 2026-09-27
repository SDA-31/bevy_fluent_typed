## Choose what stays loaded

A **module** is one FTL file. A **scope** is a generated leaf, a folder containing
leaves, or the whole `Translations` tree. Loading a scope requests all its leaves.

| Need | Plugin/controller mode | Requests |
| --- | --- | --- |
| A small catalog, keep the selected language ready | `Full` (default) | All modules automatically |
| Load screens or chapters only when needed | `Lazy` | Explicit `load` and `unload` |
| Keep a HUD ready, release optional screens | `Lazy` | Keep the HUD request; add/remove others |

“Hybrid” is a way of using Lazy, not another type. Reading an accessor or creating
a text binding never loads data. Full has no `load` or `unload` methods.

The examples below use the quickstart's `presentation/hud.ftl` and
`screens/pause.ftl`, with English and Spanish translations.

## Fully Lazy: complete main.rs

Keep the quickstart's Cargo.toml, build.rs and assets. Replace `src/main.rs` with
this program. It requests the HUD, prints `Ready` and exits. The pause module is
never requested, read or parsed:

```rust,ignore
use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    prelude::*,
};
use bevy_fluent_typed::{
    Lazy, Localization, LocalizationManifest, LocalizationPlugin, ModuleStatus,
};
use std::{path::Path, time::Duration};

bevy_fluent_typed::translations!(mod texts);
type AppLocalization = Localization<texts::Translations, Lazy>;

fn main() -> AppExit {
    let manifest = LocalizationManifest::parse(
        texts::CATALOG_CONFIG,
        texts::CATALOG_ASSET_PATH,
    )
    .expect("valid localization manifest");
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join(texts::ASSET_ROOT);

    App::new()
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(16))),
            AssetPlugin {
                file_path: assets.to_string_lossy().into_owned(),
                ..default()
            },
            LocalizationPlugin::<texts::Translations, Lazy>::new(manifest),
        ))
        .add_systems(Startup, request_hud)
        .add_systems(Update, show_title)
        .run()
}

fn request_hud(mut localization: ResMut<AppLocalization>) {
    localization.load::<texts::presentation::Hud>();
}

fn show_title(
    hud: Option<Res<texts::presentation::Hud>>,
    localization: Res<AppLocalization>,
    mut exit: MessageWriter<AppExit>,
) {
    if let Some(hud) = hud {
        println!("{}", hud.msg_title());
        exit.write(AppExit::Success);
    } else if let ModuleStatus::Failed(error) = localization.status::<texts::presentation::Hud>() {
        eprintln!("{error}");
        exit.write(AppExit::error());
    }
}
```

`LocalizationPlugin::<texts::Translations>::new_lazy(manifest)` is the shorter
initializer for the same plugin type. The controller's `Lazy` type must match the
plugin; the generated resource type is still just `texts::presentation::Hud`.

In a windowed application, install localization after `DefaultPlugins` (which
includes `AssetPlugin`). Use your normal event loop instead of this example's
`MinimalPlugins` and console exit system.

For embedded bytes, the current local 0.2.0 checkout lets this same program
select `texts::embed_manifest!(module = texts::presentation::Hud)` and keep its
existing HUD request. Follow the [embedding recipe and local setup](https://github.com/SDA-31/bevy_fluent_typed/blob/feat/runtime-module-loading/README.md#explicit-embedding):
the pinned Git snapshot above predates typed selection. Embedding chooses which
raw bytes enter the binary; Lazy requests choose which modules get parsed.
Use `texts::embed_manifest!()` to include the pause screen for the next section too.

## Load and release a screen

In the same Lazy application, call these systems when the pause screen opens and
closes. For example, register them in your state's `OnEnter` and `OnExit`
schedules; do not register both unconditionally in `Update`:

```rust,ignore
fn open_pause(mut localization: ResMut<AppLocalization>) {
    localization.load::<texts::screens::Pause>();
}

fn close_pause(mut localization: ResMut<AppLocalization>) {
    localization.unload::<texts::screens::Pause>();
}
```

Keep the HUD's request while the game uses it. This gives hybrid loading. Release
that request with `unload::<texts::presentation::Hud>()` when its owner exits too.

Requests are idempotent **per scope type**, not reference-counted calls. Two
`load::<Hud>()` calls followed by one `unload::<Hud>()` release that request. If
several application systems share ownership, coordinate that ownership in your
application. Independent parent/child requests do overlap:

```rust,ignore
localization.load::<texts::Presentation>();
localization.load::<texts::presentation::Hud>();
localization.unload::<texts::Presentation>(); // The explicit Hud request remains.
localization.unload::<texts::presentation::Hud>(); // Now it can be released.
```

`load::<texts::Translations>()` requests the whole tree. It removes the memory
benefit of partial loading while that request remains active. Subdivide very
large translations into useful FTL files: a leaf is parsed and retained as a
whole, not one message at a time.

After the last request is released, the plugin drops its scope snapshots and
strong asset handles at synchronization. Bevy may retire assets over later
updates. Application-owned clones can keep data alive, and explicitly embedded
static bytes always remain in the executable. There is no language cache or
implicit fallback.

## Read resources and handle readiness

The plugin publishes each ready leaf as a Bevy resource. A parent resource exists
only when all its children are ready; `Translations` exists only when the whole
tree is ready. A HUD resource does not depend on the pause module.

For a system that should keep running while text loads, use an optional resource:

```rust,ignore
fn use_hud(hud: Option<Res<texts::presentation::Hud>>) {
    let Some(hud) = hud else {
        return;
    };

    println!("{}", hud.msg_title());
}
```

For a system that needs the resource on every run, guard its registration:

```rust,ignore
fn use_hud(hud: Res<texts::presentation::Hud>) {
    println!("{}", hud.msg_title());
}

// In your App setup:
app.add_systems(Update, use_hud.run_if(resource_exists::<texts::presentation::Hud>));
```

Do not require loaded resources in `Startup`. Even embedded translations are
parsed during updates. `Localization::new(locale)` and `Default` create a
controller, not a ready catalog.

Use `status::<Scope>()` to distinguish `Unloaded`, `Loading`, `Ready` and
`Failed(error)`. An invalid same-language reload preserves the last good value:
its resource can be available while the latest attempt has status `Failed`.
Repeated `load::<Scope>()` on a failed request retries its failed leaves.

## Navigate from the root or a parent

The controller's views work with a partially loaded tree:

```rust,ignore
fn inspect(localization: Res<AppLocalization>) {
    let presentation = localization.modules().presentation();

    if let Ok(hud) = presentation.hud() {
        println!("{}", hud.msg_title());
    }
}
```

Group methods return another view. Leaf methods return `Result<&Leaf,
ModuleError>` with the locale, logical module path and loading status. Views do
not load anything. A leaf borrow belongs to the controller's store, so a chained
call such as `localization.modules().presentation().hud()` is also valid.

With a complete scope resource, ordinary accessors return direct references:

```rust,ignore
fn inspect_complete(presentation: Res<texts::Presentation>) {
    let hud = presentation.hud();
    println!("{}", hud.msg_title());
}
```

Guard that system with `resource_exists::<texts::Presentation>` too.
`localization.catalog()` returns `Option<&texts::Translations>`; it is normally
`None` in an application that intentionally leaves some modules unloaded.

## Bind text without keeping an old translation

A `LocalizedText<Scope>` component stores a closure. The plugin renders it using
the current resource and updates the existing `Text` or `Text2d` component when
the language, module or binding changes:

```rust,ignore
use bevy_fluent_typed::LocalizedText;

fn spawn_title(mut commands: Commands) {
    commands.spawn((
        Text::default(),
        LocalizedText::<texts::presentation::Hud>::new(|hud| hud.msg_title()),
    ));
}
```

Register `spawn_title` in `Startup`; it is safe to create a binding before its
module loads. Your application still requests the HUD and owns the usual Bevy
UI/camera/font setup. Missing or unloaded resources clear bound text, and it
refreshes when the resource becomes available. Bind to the smallest scope the
message uses: a `LocalizedText<Translations>` would wait for the entire tree.

For stored notices, use `Message<Scope>` and render only when its scope is ready:

```rust,ignore
use bevy_fluent_typed::Message;

let message = Message::new(|hud: &texts::presentation::Hud| hud.msg_title());
// Or: Message::<texts::presentation::Hud>::new(|hud| hud.msg_title())

if let Ok(hud) = localization.modules().presentation().hud() {
    let text = message.render(hud);
}
```

Capture owned message arguments, not a translated string or an old module
snapshot. Replace the binding when captured values change. Editable text drafts
should use separate components: localization replaces bound text. Number
formatting stays application-owned; see the ICU4X example.

## Change language and reload files

For the quickstart's locales and Lazy type alias:

```rust,ignore
fn select_spanish(mut localization: ResMut<AppLocalization>) {
    localization.set_locale(texts::Locale::Es);
}
```

Call it from your language-selection action. Existing scope requests stay active;
previous-language data is cleared and the same requested modules load in Spanish.
Bindings update as their resources become ready. No old-language fallback is used.
For a different initial language, insert `AppLocalization::new(texts::Locale::Es)`
before installing the plugin. Otherwise the manifest's default language is used.

To retry all requested FTL files on Bevy 0.17–0.19:

```rust,ignore
use bevy_fluent_typed::ReloadCatalogs;

fn reload(mut requests: MessageWriter<ReloadCatalogs<texts::Translations>>) {
    requests.write(ReloadCatalogs::default());
}
```

Register this on your reload action, not every frame. Bevy 0.16 uses `EventWriter`
and `send`. The request works without filesystem watching and can recover from
initially missing files. It does not reread TOML: the manifest remains the
contract you passed to the plugin.

To watch files automatically, enable the runtime's `watch` feature on the normal
dependency and set `AssetPlugin::watch_for_changes_override` to `Some(true)`.
The selected asset source must support watching. Compatible prose edits can load
without recompiling; changing the schema requires rebuilding the application.

For load diagnostics, use `CatalogUpdateReader<texts::Translations>` and iterate
`.read()`. `CatalogUpdate::Loaded { locale, path }` identifies an accepted module;
`Rejected { locale, path, error }` reports a failed attempt. In `PreUpdate`, order
the observer after `LocalizationSystems::Publish`. Error presentation belongs to
the application.

## Scheduling and reload guarantees

Publication runs in PreUpdate's `LocalizationSystems::Publish`, then again in
PostUpdate before `LocalizationSystems::Refresh`, where text bindings refresh.
A language change in Update changes the controller immediately; scope resources
follow at publication. Consumers that must observe the synchronized result in
that frame should run in PostUpdate after `LocalizationSystems::Refresh`.

Checked leaves publish independently. A bad same-language reload keeps that
leaf's last good value; valid siblings can still change. There is no multi-file
transaction. Successful reloads publish fresh snapshots even if text is identical;
idle frames and unchanged siblings preserve resource identity and change ticks.

The plugin serializes its own reload requests per module and coalesces pending
retries. Watcher reloads and direct `AssetServer::reload` calls bypass that queue.
Bevy exposes no request generation before opening the reader or in a failure
event, so overlapping external reloads cannot always be ordered by request time.
If strict order matters, disable automatic watching and send `ReloadCatalogs`;
a custom reader must supply coherent data as well.

## Migration from registry 0.1.3

| Previous usage | Unreleased API |
| --- | --- |
| Plugin receives a manifest path string | Pass a prepared `LocalizationManifest` |
| Implicit embedded startup | Choose files or explicitly invoke `texts::embed_manifest!()` |
| `Locale::load` / `Translations::embedded` | Core byte constructors or explicit manifest loading |
| `localization.catalog()` is always ready | Handle `Option<&Translations>` or read a ready leaf resource |
| Every message needs the root | Use `Message<Leaf>` / `LocalizedText<Leaf>` where possible |
| Whole-language provider parsing | Per-leaf `Module` parsers and `FluentScope` assembly |

Typed message accessors and Arc-backed generated scope types remain. Direct core
constructors `Leaf::new` and `Translations::from_modules` validate by default.
Safe `_unchecked` counterparts skip schema validation, still checking UTF-8 and
Fluent syntax. Separate validation methods let applications check data without
retaining a runtime scope.

Handwritten providers implement `FluentScope` and `FluentCatalog` with checked
`Module::new::<Leaf>` parsers and `ScopeRegistration::new::<Scope>` assembly.
Assembly shares already-ready children and must not reread or reparse sources.
See `examples/no_codegen` for a complete provider. Only one plugin/controller
mode may own a given root type in an App.

## Compile-time mode boundaries

`Full` and `Lazy` implement the sealed `LoadingMode` trait. The compiler rejects
explicit requests on Full and scopes belonging to another root:

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
