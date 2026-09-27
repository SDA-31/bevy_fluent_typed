# Loading guide

Start with the [complete quickstart](README.md#setup), including its
dependencies, build script and four FTL files. The snippets below use those
generated types and Bevy 0.19.

## Contents

- [Choose Full, Lazy or hybrid](#choose-what-stays-loaded)
- [Complete Lazy application](#fully-lazy-complete-mainrs)
- [Load and release a screen](#load-and-release-a-screen)
- [Read resources](#read-resources-and-handle-readiness)
- [Navigate from a parent](#navigate-from-the-root-or-a-parent)
- [Bind text](#bind-text-without-keeping-an-old-translation)
- [Change language and reload](#change-language-and-reload-files)
- [Scheduling and guarantees](#scheduling-and-reload-guarantees)
- [Migrate from 0.1.3](#migration-from-registry-013)
- [Troubleshooting](README.md#troubleshooting)

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
    Lazy, Localization, LocalizationAppExt, LocalizationManifest, LocalizationPlugin, ModuleStatus,
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
        .add_localized_startup_systems(show_title)
        .add_systems(Update, report_failure)
        .run()
}

fn request_hud(mut localization: ResMut<AppLocalization>) {
    localization.load::<texts::presentation::Hud>();
}

fn show_title(
    hud: Res<texts::presentation::Hud>,
    mut exit: MessageWriter<AppExit>,
) {
    println!("{}", hud.msg_title());
    exit.write(AppExit::Success);
}

fn report_failure(
    localization: Res<AppLocalization>,
    mut exit: MessageWriter<AppExit>,
) {
    if let ModuleStatus::Failed(error) = localization.status::<texts::presentation::Hud>() {
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

For embedded bytes, this same program can select `texts::embed_manifest!(module = texts::presentation::Hud)` and keep its
existing HUD request. Follow the [embedding recipe](https://github.com/SDA-31/bevy_fluent_typed/blob/main/README.md#explicit-embedding):
embedding chooses which
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

Use native `Res<Scope>` when a system needs ready translations on every run:

```rust,ignore
use bevy_fluent_typed::LocalizationAppExt;

fn update_hud(hud: Res<texts::presentation::Hud>) {
    println!("{}", hud.msg_title());
}

app.add_localized_systems(Update, update_hud);
```

The library infers the required catalog types from direct `Res<Scope>` parameters.
Before loading, after unloading and during a locale transition, the system waits
without blocking the frame. It resumes when its catalogs are ready. A complete
parent waits for all its children; a HUD leaf does not wait for the pause module.
In Lazy mode, keep the explicit `load::<Scope>()` and `unload::<Scope>()` calls:
registering a system does not request or retain a module.

Functions and tuples are supported; tuple members wait independently. To add
normal Bevy scheduling configuration, wrap functions before configuring them:

```rust,ignore
use bevy_fluent_typed::localized;

app.add_systems(Update, localized(update_hud).run_if(screen_is_open));
```

For initialization that must happen once after loading:

```rust,ignore
app.add_localized_startup_systems(setup_hud);

fn setup_hud(mut commands: Commands, hud: Res<texts::presentation::Hud>) {
    commands.spawn(Text::new(hud.msg_title()));
}
```

This helper runs in `Update`, remembers actual invocation and applies normal
Bevy deferred commands. It does not rerun after unloading or language changes;
use `LocalizedText` for text that must stay live. A function returning an error
still counts as invoked, and Bevy handles that error normally. These functions
have no ordering relationship with ordinary `Startup` systems beyond running
later. Tuple members are independent. This helper wraps each function, so
`.before(setup_hud)` / `.after(setup_hud)` do not target the deferred wrapper.
Combine dependent initialization steps in one function, or use recurring
`localized(...)` systems with application-owned initialization state and ordering.

If a system should keep working before its translations arrive, keep `Option`:

```rust,ignore
fn observe_hud(hud: Option<Res<texts::presentation::Hud>>) {
    if let Some(hud) = hud {
        println!("{}", hud.msg_title());
    }
}

app.add_systems(Update, observe_hud);
```

Optional parameters never delay a system, including when used beside a required
catalog in `add_localized_systems`. Missing ordinary resources retain Bevy's
normal validation behavior; the helper does not suppress unrelated errors.

Readiness inference supports direct native `Res<Scope>` parameters of functions
and closures. Custom derived `SystemParam`s, `ParamSet` and nested parameter tuples
are not inspected: expose each required catalog as a direct parameter or retain
explicit conditions for those advanced forms. Apply configuration after
`localized`; already configured systems have erased their parameter types.

The ordinary `add_systems` API is unchanged. The new helper requires a recurring
schedule, such as `Update`, to retry waiting systems. Ordinary `Startup`, `OnEnter`
and other one-shot schedules do not retry; use deferred initialization above when
needed. `add_localized_systems` rejects the three built-in startup schedules to
avoid silently losing an initialization step.

`Localization::new(locale)` and `Default` create a controller, not a ready catalog.
Use `status::<Scope>()` to distinguish `Unloaded`, `Loading`, `Ready` and
`Failed(error)`. A separate observer can report load errors while required systems
wait. An invalid same-language reload preserves the last good value, so its
consumers can run while the latest attempt has status `Failed`. Repeating
`load::<Scope>()` retries failed leaves.

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

Register that system with `app.add_localized_systems(Update, inspect_complete)`.
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

Follow the [0.2.0 migration guide](docs/migration-0.2.md) for dependency updates, before/after
initialization, readiness handling, optional Lazy adoption and handwritten providers.
The shortest upgrade keeps Full mode and existing typed message calls.

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

## More recipes

- [Custom sources](docs/asset-sources.md): supply files through your own reader.
- [Decimal and percentage arguments](docs/formatting.md): reuse ICU4X formatters.
- [Build API](docs/build.md): explicit generation and feature selection.
