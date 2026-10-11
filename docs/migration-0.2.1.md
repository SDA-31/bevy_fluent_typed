# Adopt native resource waiting

Use runtime 0.3.1 with matching normal and build dependencies. On Bevy 0.19/0.20,
ordinary registrations discover catalog demand and wait for native `Res<Scope>`:

```rust,ignore
fn update_hud(hud: Res<texts::presentation::Hud>) {
    println!("{}", hud.msg_title());
}

app.add_systems(Update, update_hud.run_if(screen_is_open));
app.add_systems(Startup, setup_hud);
```

Required catalog reads, including aliases and derived parameters, are discovered
from scheduler metadata. In default Lazy, they request and retain their scopes.
A recurring consumer waits without blocking the frame; normal Bevy ordering,
sets, conditions and deferred commands remain available. Default Deferred startup
runs ready independent steps once, waits for catalog consumers and their declared
dependents, then proceeds to later startup phases.

Use the generated `texts::manifest()` contract prepared by the explicit build call.
The helper reads no files and embeds no FTL. Existing explicit manifests and
`embed_manifest!` remain available for different runtime storage.

See the [0.3.1 migration](migration-0.3.1.md) for startup strategies and the
[resource reference](../GUIDE.md#read-resources-and-handle-readiness) for schedule
discovery boundaries, passive observers and older-backend helpers.

## Older Bevy backends

Bevy 0.16–0.18 keep `LocalizationAppExt::add_localized_systems` and `localized`
for recurring functions with direct required catalog parameters.
`add_localized_startup_systems` provides independent one-time work in Update,
without preserving original startup ordering references. Their ordinary startup
and custom one-shot schedules do not retry waiting catalog consumers.
These helpers remain available for existing 0.19/0.20 code, but are unnecessary
for ordinary native registration on those backends.

## Keep explicit loading when intended

Manual still requires application-owned `load`/`unload` or leases; readiness does
not create demand in that mode. Full requests the complete selected language.
Same-language reload failures preserve the last good resources. Use `status` and
`CatalogUpdate` in a separate system to observe failed loads and explicitly retry.
Bindings follow locale switches; a string copied into `Text` during startup does not.

## Byte sources and optional manifest support

For catalog path and selective embedding changes, follow the
[0.2.2 migration](migration-0.2.2.md). The build adapter remains inside the public
runtime crate: no direct bridge dependency is needed. Backend-only handwritten
providers support bytes without the generator; add `manifest` for TOML constructors.
See the [feature table](build.md#features).
