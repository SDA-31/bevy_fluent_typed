The example uses embedded catalogs immediately. External asset loading is
asynchronous; this short example does not wait for it to finish. The separate
`localization-example` binary waits for external catalogs and supports `--watch`.

## Running the examples

From a checkout of [the repository](https://github.com/SDA-31/bevy_fluent_typed):

```sh
cargo run --manifest-path examples/minimal/Cargo.toml --bin typed_resources --config 'patch.crates-io.fluent_typed_codegen.git="https://github.com/SDA-31/fluent_typed_codegen.git"' --config 'patch.crates-io.fluent_typed_codegen.branch="main"'
cargo run --manifest-path examples/minimal/Cargo.toml --bin localization-example --config 'patch.crates-io.fluent_typed_codegen.git="https://github.com/SDA-31/fluent_typed_codegen.git"' --config 'patch.crates-io.fluent_typed_codegen.branch="main"' -- --watch
```

Use `--locked --offline` on later runs once dependencies and a lockfile exist.
Both examples run headlessly, without a GPU or window.

## Direct resources or deferred text?

- **Direct module resource:** `Res<texts::presentation::Hud>` gives a system only
  its own module's typed methods, suitable for immediate-mode UI or explicit
  formatting. Root, folder and file resources share immutable catalogs via `Arc`.
- **Chained borrow:** pass `&texts::Presentation` or `&texts::presentation::Hud`
  to an ordinary function without exposing the whole translation tree.
- **[`LocalizedText`]:** attach a deferred formatter to an existing Bevy `Text`
  or `Text2d`. The plugin updates it after language changes and accepted reloads.
  Capture owned arguments, not a previously translated string; replace the
  component when its arguments change.
- **[`Message`]:** keep deferred formatting without binding a text entity.

Change language through [`Localization::set_locale`], not individual module
resources. Initialize `Localization::<Translations>::new(Locale::Es)` **before**
adding the plugin to override startup language.

Catalog immutability does not freeze displayed text. Replace a `LocalizedText`
binding to choose another message, change captured arguments or transform its
formatted result. Editable user drafts should use separate text components:
direct edits to a bound `Text`/`Text2d` can be overwritten by the next binding or
catalog refresh. In-memory editing of shared FTL templates is not currently a
public runtime API; it would need checked, whole-catalog publication.

## Hot reload and failures

Enable `watch` and Bevy's asset watcher. For the generated provider, compatible
FTL prose edits reload without restarting. Changes to languages, modules, keys,
references, argument contracts or configuration need regeneration and restart.
Custom providers define their own compatibility policy. IDE regeneration and
runtime hot reload are separate operations.

A valid **whole-language** snapshot replaces its last-known-good catalog. Invalid
or unchanged edits leave that snapshot intact. One language may update while
another fails validation; separate file saves are not one transaction. Embedded
catalogs remain usable if external assets are absent or unreadable.

Read [`CatalogUpdate`] after [`LocalizationSystems::Publish`] for success or
failure details. A `Rejected` event with `locale: None` means the definition or
aggregate load failed; a locale identifies a rejected language candidate.
`Loaded` does not necessarily mean the catalog changed.

Send [`ReloadCatalogs`] to retry manually, including without file watching. A file
that was read but rejected stays watched and can recover after correction. An
initially missing file needs an explicit reload after it is created.

## Schedule and ownership

The plugin publishes embedded module resources immediately, before Startup.
[`LocalizationSystems::Publish`] runs in PreUpdate, and publication also precedes
PostUpdate's [`LocalizationSystems::Refresh`] text update.

If Update systems need newly selected module resources, select the language in
PreUpdate before Publish. A switch during Update reaches direct module resources
during PostUpdate's Refresh set; order PostUpdate consumers with
`.after(LocalizationSystems::Refresh)` to observe the synchronized resources and
text. The central `Localization` may be newer during that Update.
Unchanged snapshots do not replace the direct module resources.

The runtime owns asset loading, language state, publication and text bindings.
It does not interpret the directory generator's TOML. The optional bridge owns
that adapter; without it, implement [`FluentCatalog`] with your own checked parser,
definition format and compatibility policy. The application owns fonts, layout,
input, numeric formatting and how errors are presented to users.

## More detail

- [Full guide: configuration, contracts, custom providers and scheduling](https://github.com/SDA-31/bevy_fluent_typed/blob/main/GUIDE.md)
- [Headless examples and their assets](https://github.com/SDA-31/bevy_fluent_typed/tree/main/examples/minimal)
- [Optional codegen bridge](https://github.com/SDA-31/bevy_fluent_typed/tree/main/codegen_bridge)
- [Independent directory generator](https://github.com/SDA-31/fluent_typed_codegen)

These repository links follow `main`; the API reference on this page describes
the crate version being viewed. MIT covers this library and its examples, not
your application or its translation assets.
