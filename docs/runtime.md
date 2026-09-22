The example uses embedded catalogs immediately. External asset loading is
asynchronous; this short example does not wait for it to finish. The separate
`localization-example` binary waits for external catalogs and supports `--watch`.

## Running the examples

From a checkout of [the repository](https://github.com/SDA-31/bevy_fluent_typed):

```sh
cargo run --manifest-path examples/minimal/Cargo.toml --bin typed_resources
cargo run --manifest-path examples/minimal/Cargo.toml --bin localization-example -- --watch
cargo run --manifest-path examples/codegen/Cargo.toml
cargo run --manifest-path examples/no_codegen/Cargo.toml
cargo run --manifest-path examples/icu/Cargo.toml
cargo run --manifest-path examples/asset_source/Cargo.toml
```

Use `--locked --offline` on later runs once dependencies and a lockfile exist.
All examples run headlessly, without a GPU or window. `codegen` is the smallest
generated consumer with an explicit build.rs; `no_codegen` has a handwritten
provider and no build script. The larger `minimal` remains the integration suite.

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

## Decimal values and plural selection

Use dedicated [ICU4X](https://docs.rs/icu/) or
[ICU](https://unicode-org.github.io/icu/userguide/format_parse/) components for
numbers, percentages, currencies and dates; their APIs and stability differ.
ICU4X DecimalFormatter and PluralRules can supply text and a plural-category keyword
to two generated String arguments. Prepare display precision once for both; Fluent
matches the keyword to literal branches such as `[one]` or `[few]`. Native numeric
selectors remain available, including exact `[0]` / `[1]` matches. The runtime
does not add an ICU dependency or require a numeric-formatting feature.

For deferred messages, capture a Decimal and reusable per-locale formatters;
choose the formatter from the current catalog's locale when rendering. Capturing
an already formatted string would keep the old language's digits and
grammar. Replace a binding when its numeric input or precision policy changes.
The [plural guide and FTL contract](https://github.com/SDA-31/bevy_fluent_typed/blob/main/GUIDE.md#decimal-and-plural-arguments)
and [compiled text-switch regression](https://github.com/SDA-31/bevy_fluent_typed/blob/main/examples/minimal/tests/catalog/plurals.rs)
show the two String arguments and their lifecycle.

Arabic number formatting is separate from visual RTL: preserve Fluent's default
bidi isolation, and provide glyph shaping, bidi layout, fonts and UI mirroring
through your rendering stack. These headless examples verify strings, not pixels.

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
failure details using [`CatalogUpdateReader`] and its `.read()` iterator. This is
a Bevy `EventReader` on 0.16 and a `MessageReader` on newer backends, not another
queue or an extra processing step. A `Rejected` event with `locale: None` means
the definition or aggregate load failed; a locale identifies a rejected language candidate.
`Loaded` does not necessarily mean the catalog changed.

Send [`ReloadCatalogs`] to retry manually, including without file watching. A file
that was read but rejected stays watched and can recover after correction. An
initially missing file needs an explicit reload after it is created.
On 0.16, send it through `EventWriter` / `World::send_event`; on newer backends,
use `MessageWriter` / `World::write_message`.

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
