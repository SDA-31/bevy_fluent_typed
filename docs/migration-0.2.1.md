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
discovery boundaries, passive observers and older-backend registration helpers.

## Older Bevy backends

Bevy 0.16–0.18 retain `LocalizationAppExt` and `localized` for direct required
catalog reads, including automatic Lazy demand and readiness. Their one-time
helper runs in Update; native one-shot startup does not retry an absent catalog. See the [older-backend recipe](../GUIDE.md#older-bevy-backends).
`Localized` still loads and retains its scope automatically in Lazy on all
backends. The scheduling helpers are absent on Bevy 0.19/0.20.

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
