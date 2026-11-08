# Migrate from bevy_fluent_typed 0.4.0 to 0.5.0

This release uses fluent-typed 0.10.1 and fluent_typed_codegen 0.3.0. The Rust
minimum remains 1.97.1; the same Bevy 0.16.1–0.20 backends remain supported.

[Migration Guide](README.md)

## Update both facade dependencies

```toml
[dependencies]
bevy_fluent_typed = { version = "0.5.0", features = ["codegen"] }

[build-dependencies]
bevy_fluent_typed = { version = "0.5.0", default-features = false, features = ["build"] }
```

Keep build.rs, translations! and the existing manifest setup. Select a different
backend only on the normal dependency, with default features disabled. No engine
feature is needed on the build dependency. Handwritten providers retain their
backend-only graph without the generator.

If custom build extensions also depend directly on `fluent_typed_codegen`, update
that dependency and any matching workspace patch to 0.3.0. Ordinary Bevy consumers
need only the facade dependencies above.

## Update structured accessors

`(Element)` methods now use the same `msg_` prefix as plain messages. Replace
`options.prompt()` with `options.msg_prompt()`. The structured return
type and fields stay the same. This also applies inside `Msg` and `Localized` closures.

```rust,ignore
// Before
let prompt = options.prompt();
// After
let prompt = options.msg_prompt();
```

Additional upstream contract checks can reveal invalid translations as build
warnings and absent accessors. Fix referenced messages/attributes/terms,
unsupported functions or argument usage in FTL. Checked runtime loads keep the
previous catalog active when a new candidate is rejected.

## Remove duplicate Debug extensions

Generated catalog scopes now derive `Debug`. If a codegen extension previously
added `Debug` to those same scopes or emitted a manual implementation, remove
that duplicate implementation. Application types are unaffected.

## Refresh an affected executor lock

If Cargo.lock still contains async-executor 1.14.0, update that entry:

```sh
cargo update -p async-executor --precise 1.14.1
```

Version 1.14.0 was yanked after an executor-drop use-after-free regression;
1.14.1 contains the [upstream fix](https://github.com/smol-rs/async-executor/blob/master/CHANGELOG.md#version-1141).
Repository and example locks already use the fixed patch. This is a transitive
executor fix; the localization API and compiler minimum are unchanged.

## Transfer buffers through the standard constructors

Generated `new` / `new_unchecked` and `LocalizationPlugin::from_loader` now use
`Into<Vec<u8>>`, matching fluent-typed. A fresh Vec<u8> or String transfers its
allocation. Slices are copied; change `new(locale, &bytes)` with a Vec input to
`new(locale, bytes.as_slice())` when retaining that buffer. Custom callback result
types previously implementing only AsRef<[u8]> must convert explicitly to Vec<u8>
or implement that owned conversion.

For a fresh source buffer, transfer it directly instead of borrowing it:

```rust,ignore
// Before: an AsRef-based loader could return a shared custom wrapper.
LocalizationPlugin::<texts::Translations>::from_loader(|locale, path| async move {
    read_source(locale, path).await // Result<SharedBytes, Error>
});
// After: convert a shared wrapper, or return the fresh Vec directly.
LocalizationPlugin::<texts::Translations>::from_loader(|locale, path| async move {
    read_source(locale, path).await.map(|bytes| bytes.as_ref().to_vec())
});
```

Do not add that copy when the source already returns `Vec<u8>` or `String`:
return the owned value as-is. `&[u8]` is also accepted, with a copy into a Vec.

`Module::new` now passes `Vec<u8>` to handwritten checked parsers. If a parser
has an explicitly annotated `bytes: &[u8]` parameter, change it to `Vec<u8>`:

```rust,ignore
Module::new::<Hud>("ui/hud.ftl", |locale, bytes| {
    Hud::new(locale, bytes).map_err(|error| error.to_string())
})
```

Pass the Vec into an ownership-aware parser, or borrow it locally, for example
`std::str::from_utf8(&bytes)`. The constructor still only registers the parser;
source acquisition and parsing happen when the module is requested.
Generated providers already use this contract. See
[custom byte sources](../runtime.md#custom-byte-sources) for source configuration
and error handling.

Async scheduling, cancellation, retries and publication retain their contracts.
Callback futures must be nonblocking. from_bytes keeps shared input for reuse,
and embedded input remains static; each supplies an owned copy for parsing.
This removes a temporary full-source copy for fresh buffers and does not change
rendered-output cache policy.

## Retained behavior and verification

`Localized` still requires an application-supplied target; no initialization hook
or `Default` requirement was added. If the target is missing, the runtime warns
and removes the orphaned binding; it does not create the target. Loading modes,
provider registration, startup,
locale preparation/commit and the binding lifecycle keep their existing contracts.

Run cargo test --locked in the application, then test one language change,
structured text, a compatible reload and rejection/recovery of invalid input.
When using a custom source, verify cancellation and reacquisition with that source.

## Check names reported during generation

fluent-typed 0.10.1 rejects message, argument and locale names that collide after
Rust-name normalization. Fix the locations reported by generation. Locale
folders must be canonical language identifiers with two- or three-letter
language subtags. Hidden directories such as `.history` are ignored; symbolic
links remain unsupported. These checks run during the explicit build step.
