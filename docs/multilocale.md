# Render several languages

This page describes the experimental API in this checkout. Its names and details
may change before integration; it is not part of a published release.

Use this API to show language names in their own languages, or keep an original
subtitle beside one following the game's language. Each root provider continues
owning one source and one cache policy. Additional languages do not require
additional localization plugins.

A binding follows the active language by default. Fix its language explicitly:

```rust,ignore
let message = Msg::<Dialog>::new(|dialog| dialog.msg_line());
commands.spawn((Text::default(), Localized::<Text, _>::from(message.clone())));
let fixed = commands
    .spawn((
        Text::default(),
        Localized::<Text, _>::from(message.clone()).with_locale(Locale::Ru),
    ))
    .id();
```

Keep `Msg` as the reusable message definition; `Localized` is the final binding
attached to a target. `with_locale` preserves automatic message and catalog
updates. To change the selected language, replace the binding from the retained
message. Omitting the modifier makes the new binding follow the provider again:

```rust,ignore
commands.entity(fixed).insert(Localized::<Text, _>::from(message));
```

The target component remains application-owned. Replacing `Localized` preserves
that target and other entities' bindings. Replace the message as well when its
captured arguments change.

Native BSN on Bevy 0.20 accepts the modifier inside the scene:

```rust,ignore
bsn! {
    Text
    Localized::<Text, Dialog>::from(message).with_locale(Locale::Ru)
}
```

On Bevy 0.19, configure the binding in ordinary Rust before passing it through
the typed constructor. This form also works on 0.20:

```rust,ignore
let binding = Localized::<Text, Dialog>::from(message).with_locale(Locale::Ru);
bsn! {
    Text
    Localized::<Text, Dialog>::from(binding)
}
```

Fixed bindings still react to translation reloads; removing the last one
releases its language's demand.
The [BSN locale tests](../examples/bsn/src/tests/locales.rs) cover native syntax,
fixed/following labels, cloned binding conversion, replacement from a retained
message, reload/release and different languages for Text, Text2d, TextSpan and
AccessibleLabel.

For application-owned strings, retain your own `Msg<Dialog>` and request its
scope in a fixed language:

```rust,ignore
localization.for_locale_mut(Locale::Ru).load::<Dialog>();
```

After the scope is ready, use the ordinary generated navigation and message:

```rust,ignore
let dialog = localization.for_locale(Locale::Ru).modules().dialog()?;
let text = message.render(dialog);
```

Reading never loads modules. Missing data uses the existing `ModuleError`, with
its language, module path and latest attempt status. `status` and `progress` on
the read-only language view inspect readiness. A root is complete only when all
its modules are ready; prefer the smallest scope needed by the screen.

Explicit fixed-language requests work with every loading mode. `load` is an
idempotent fixed-language pin, released by the corresponding
`unload`. For independent ownership, use the language view's `hold` and retain
its `ModuleLease` until the screen closes. Dropping the view does not release a
pin or a lease. Existing `Localization::load/hold` continue following the active
language. Overlapping requests share acquisition while retaining independent
ownership.

Lazy bindings retain their selected language and scope. Manual bindings still
require explicit requests. Full retains the complete active-language root;
fixed bindings additionally retain their selected scopes. Neither inspection
nor an unused compiled language triggers loading. Same-language reload failures
keep the last usable snapshot. An unavailable binding uses the existing empty
value behavior, including while a newly selected fixed language is loading.

`set_locale` still publishes the new active language only when its following
consumers are ready. Fixed consumers keep their languages. An unrelated pending
fixed module does not block this publication. `Res<Scope>` continues exposing
only the active language. CachePolicy remains a policy for formatted binding
output; it does not disable retention of requested parsed catalogs.

Native root progress counts demand for the active language, including its fixed
consumers. Preparation progress follows the consumers moving with the game
language. Inspect another language through its view's per-scope `progress`.

Replace a binding normally when selecting another language. A write through
`bypass_change_detection` does not update its automatic language demand.
The existing message-identity invalidation remains separate from that demand.
