# Text and application component bindings

One generated catalog, English/Spanish/Russian, ordinary Bevy spawning. Run from
the runtime repository:

```sh
cargo run --locked --manifest-path examples/bindings/Cargo.toml
cargo run --locked --manifest-path examples/bindings/Cargo.toml -- --no-cache
cargo test --locked --manifest-path examples/bindings/Cargo.toml
```

In an enclosing workspace, use `-p localization-bindings-example` instead of the
manifest path. This example uses Bevy 0.20 by default and also supports 0.19.
For that backend, change both normal dependencies:

```toml
[dependencies]
bevy = { version = "0.19.0", default-features = false, features = ["bevy_ui_widgets"] }
bevy_fluent_typed = { path = "../..", default-features = false, features = ["bevy-0-19", "codegen"] }
```

Leave the build dependency unchanged. Refresh the applicable `Cargo.lock` after
changing the manifest, before running the commands above with `--locked`.
For external applications use the
[quickstart](../../README.md#setup); local paths here exercise this checkout.
For simultaneous generator development, use the [override](../../docs/build.md#work-on-local-checkouts).

## Automatic bindings

[main.rs](src/main.rs) adds the source plugin and an ordinary Startup system.
Native and custom targets use the same `Localized<Target, Scope>` binding.
[widgets.rs](src/widgets.rs) contains the spawning code:

```rust,ignore
let message = Msg::<Greeting>::new(|greeting| greeting.msg_detail("Ada"));

commands.spawn((
    Text::default(),
    TextColor(Color::srgb(0.2, 0.8, 0.9)),
    Localized::<Text, Greeting>::from(message.clone()),
));

commands.spawn((
    Text2d::default(),
    Localized::<Text2d, Greeting>::from(message),
));
```

The first argument is the target. Bindings update `Text`, `Text2d`, `TextSpan`
and `AccessibleLabel` independently; `Localized<Text, _>` never adds an
accessibility label. Built-in targets implement `Default`, allowing a binding
to create a missing target. Explicit components preserve your styling and layout.
The span belongs to a text hierarchy supplied by the application.

The ordinary button has a `Node`, `AccessibleLabel` and its binding on the root,
and a child with `Text` and its binding. Bevy owns its layout and interaction;
these two bindings own its translated name and visible caption. BSN is optional;
see its [separate example](../bsn) for scene syntax and composition.

[targets.rs](src/widgets/targets.rs) demonstrates three levels of integration:

- `Badge` implements `LocalizationTarget` for one String field. It declares
  required mutable data, creates a missing Default target once and uses the
  public change-tracking and `set_text` helpers for direct query writes.
- `Caption` uses one Query with required mutable Caption/CaptionMirror data and
  `Without<CaptionPaused>`. Its pure `needs_update` skips Idle, and a mirror-only
  edit is repaired from Caption without reading Msg. Its initializer supplies
  missing Default components once; removing either component later is respected.
- `Card` has two fields and no Default. The application constructs it explicitly.
  CardTitle/CardDetail declare required Card data and implement only `apply`;
  the default update requests that sequential writer. They wait while Card is
  absent and compare before writing to preserve equal-output ticks.

Custom targets need no registration or extra plugin; see
[custom component bindings](../../docs/bindings.md#bind-a-custom-component).
The optional public hooks support direct query writes, lazy structural work and
post-batch confirmation. All targets share the same metadata and executor;
there are no auxiliary user Query parameters or arbitrary SystemParam hook.

The headless [driver](src/scenario.rs) switches languages without recreating
entities, replaces the Caption and CardDetail messages with owned `Grace`
arguments, and leaves the other bindings alone. It also pauses Caption, replaces
its Msg while excluded, then verifies that reentry catches up. It removes consumers separately,
leaving the existing Caption intact after removing its binding, then verifies
that removing the last binding owner releases the catalog in default Lazy.
There is no explicit load call or user refresh system.

`--no-cache` runs this same scenario with
`LocalizationPlugin::with_cache_policy(CachePolicy::None)`, inherited by every
scope in the provider. Without it, `Rendered` retains formatter output for
repair. None keeps native identity/tick metadata but no extra formatted string;
native text formats again for application edits to an existing target.
Caption skips Idle and repairs its mirror from the existing component; the simpler
Card field adapters read text every eligible refresh and therefore format every
refresh under None. AccessibleLabel follows Bevy's ordinary hooks; node-only edits
and removal do not trigger a separate localization repair. Loading and language-switch behavior are identical. See
[cache policy tradeoffs](../../docs/bindings.md#choose-a-providers-cache-policy).

The program runs without a window, GPU or font assets. Its loops, deadlines and
assertions live in the scenario driver, outside the integration code. It verifies
ECS strings, accessibility nodes, ownership and query-filter reentry; it does not
exercise rendering, button input, a screen reader or filesystem watching. The
[process test](tests/bindings.rs) runs the automatic scenario under both policies
with a deadline.

[MIT](LICENSE), including the translation fixtures.
