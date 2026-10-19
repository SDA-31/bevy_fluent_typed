# Performance and measurement scope

Automatic bindings avoid unnecessary formatting and target writes, while
retaining metadata for catalog readiness, Msg identity and query eligibility.
This page describes the 0.4.0 contract and measured refresh costs;
the [changelog](../CHANGELOG.md#040) records release status.

## Work skipped by automatic bindings

For existing, eligible built-in text targets, a ready unchanged catalog and the
default `CachePolicy::Rendered`:

| Refresh scenario | Formatter calls |
| --- | ---: |
| No relevant input or target change | 0 |
| Replace an existing binding with a clone of its current Msg | 0 |
| Replace one binding with a new Msg identity | 1 for that binding |
| Repair an ordinary application edit of the target text | 0 additional calls; reuse retained output |

Creating a new Msg establishes a new identity even if its closure returns the
same string. Cloning preserves identity; attaching that clone to another entity
creates an independent binding and result cache. Catalog publication invalidates
the selected scope, not individual Fluent keys. Direct `Msg::render` invokes
the formatter on every call and is outside this binding cache.

Built-in targets compare output before marking their component changed.
Custom adapters can use the same `LocalizationUpdate::set_text` helper, avoiding
redundant change notifications to downstream Bevy systems. Other application or
engine writes can still invalidate text and layout independently.

`CachePolicy::None` removes persistent formatted output from binding metadata.
Target components still own their strings. Built-ins skip unchanged targets;
repairs format again. Custom adapters decide when to read text and can inspect
`is_changed` and `component_changed` first. Repeated reads within an update and
its deferred continuation reuse the result. An apply-only adapter that always
reads text formats on every eligible refresh under None.

Small eligible updates run sequentially. Large updates can use Bevy's native
query batching and existing compute pool; structural callbacks remain sequential.
Catalog source versions are captured once per pass. When they are unchanged,
selection skips snapshot-version lookups for rows whose Msg, locale and query
eligibility remain current. Target change observation still runs; adapters that
request an update resolve the selected catalog and its version together.
Tracking uses compact optional snapshot identities and the existing shared
language-demand token, without another ECS component.
The pass still visits binding metadata, and parallel work completes before the
refresh returns. There is no startup cost calibration, frame budget or automatic
spreading across frames. See the [binding contract](bindings.md) for lifecycle,
filter reentry, bypass-change-detection and accessibility limits.

Parallel query execution requires Bevy's native `multi_threaded` feature.
A multi-worker compute pool alone does not enable parallel ECS queries.

## Measured refresh costs

These measurements describe the 0.4.0 implementation, measured on
October 20, 2026: Ryzen 5 4600H, Rust 1.99.0, exact Bevy 0.20.0,
release opt-level 3 and a six-worker compute pool with native multithreading.
Bindings share one catalog scope. The fixture uses a six-variant enum locale,
matching generated applications, and supplies already-ready catalogs directly.

| Text bindings | Scenario | Cache policy | Median elapsed time, ms | 95th-percentile elapsed time, ms | Mean aggregate CPU time, ms |
| ---: | --- | --- | ---: | ---: | ---: |
| 1,024 | Fluent formatting | Rendered | 0.364 | 0.388 | 0.371 |
| 100,000 | Unchanged targets | Rendered | 0.858 | 0.955 | 0.921 |
| 100,000 | String-clone formatter | Rendered | 2.435 | 4.230 | 12.772 |
| 100,000 | Fluent formatting | Rendered | 10.932 | 13.941 | 67.005 |
| 100,000 | Reattach the same Msg | Rendered | 0.853 | 1.008 | 0.885 |
| 100,000 | Replace one Msg | Rendered | 0.852 | 1.000 | 0.911 |
| 100,000 | Forced long-string growth | Rendered | 57.114 | 61.251 | 262.444 |
| 100,000 | Repair ordinary text edits | Rendered | 40.616 | 47.520 | 178.864 |
| 1,048,576 | Unchanged targets | Rendered | 9.325 | 9.735 | 9.366 |
| 1,048,576 | String-clone formatter | Rendered | 30.127 | 31.220 | 135.609 |
| 1,048,576 | Fluent formatting | Rendered | 129.558 | 144.366 | 669.227 |
| 100,000 | Unchanged targets | None | 0.892 | 1.091 | 0.894 |
| 100,000 | Fluent formatting | None | 10.037 | 11.283 | 54.684 |
| 100,000 | Forced long-string growth | None | 32.107 | 36.154 | 143.403 |
| 100,000 | Repair ordinary text edits | None | 60.954 | 67.769 | 222.369 |
| 100,000 | Unchanged targets across six fixed locale slots | Rendered | 0.858 | 0.997 | 0.926 |
| 100,000 | Fluent formatting across six fixed locale slots | Rendered | 11.236 | 12.031 | 54.774 |

For ten unchanged bindings, a separate block control measured 1.67 µs
elapsed and 1.67 µs aggregate CPU per refresh. This is the median of
five process means, each from fifteen blocks of 2,048 refreshes; batching
amortizes CPU-clock overhead for this small workload.

Elapsed time measures each synchronous `PostUpdate` refresh from start to
completion, including waiting for worker tasks. The table takes the median
of three process medians; the percentile column likewise reports the median
of three within-process percentiles. Each process records at least 64 refreshes
for the main workloads and 32 at one million targets. Fast workloads continue
toward a 150 ms observation window, with a 4,096-sample cap that can end it
earlier. Warmup is excluded; measured outliers are retained. These
are descriptive samples, not confidence intervals or worst-case bounds.

CPU is recorded in a separate trial. It sums all process threads during the
refresh interval, including the caller and workers. Each process contributes
its mean, and the table reports the median of those three means. CPU time is
not added to elapsed time; lower latency can require more aggregate CPU.

Spawn, input preparation, catalog loading/demand coordination, layout,
renderer and validation are outside the refresh timer. This is an isolated
binding-update benchmark, not a complete `App::update` or frame measurement.
The growth case deliberately clears and shrinks target buffers before each
refresh, forcing long outputs to allocate again; it is a stress workload.
The six-locale case republishes six ready snapshots with identical Fluent
payloads and distributes bindings across them; it measures selection and
refresh, not translation loading.

Actual frame impact also includes text layout and other systems competing
for the same workers. Profile a representative application, including
locale switches and ordinary UI changes, before tuning work or choosing
a cache policy. Callback placement and cost can change scheduling behavior.
