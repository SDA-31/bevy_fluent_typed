# Performance and measurement scope

Automatic bindings avoid unnecessary formatting and target writes, while
retaining metadata for catalog readiness, Msg identity and query eligibility.
This page describes the refresh contract and measured refresh costs;
the [changelog](../CHANGELOG.md) records the changes in each release.

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

These refresh measurements describe the 0.5.0 implementation, measured on
November 8, 2026: Ryzen 5 4600H, Rust 1.99.1, exact Bevy 0.20.0,
release opt-level 3 and a six-worker compute pool with native multithreading.
Bindings share one catalog scope and a cloned Msg. The selected provider policy
applies to every binding; each updates its own target and retains its own output
when caching is enabled. The fixture uses a six-variant enum locale,
matching generated applications, and supplies already-ready catalogs directly.

| Text bindings | Scenario | Cache policy | Median elapsed time, ms | 95th-percentile elapsed time, ms | Mean aggregate CPU time, ms |
| ---: | --- | --- | ---: | ---: | ---: |
| 1,024 | Fluent formatting | None | 0.352 | 0.412 | 0.372 |
| 1,024 | Fluent formatting | Rendered | 0.370 | 0.393 | 0.383 |
| 100,000 | String-clone formatter | None | 1.777 | 3.442 | 10.308 |
| 100,000 | String-clone formatter | Rendered | 2.871 | 4.109 | 13.436 |
| 100,000 | Fluent formatting | None | 7.412 | 9.171 | 38.018 |
| 100,000 | Fluent formatting (six fixed languages) | Rendered | 9.213 | 10.855 | 45.165 |
| 100,000 | Fluent formatting | Rendered | 9.712 | 10.674 | 49.415 |
| 100,000 | Forced long-string growth | None | 29.836 | 34.364 | 135.392 |
| 100,000 | Forced long-string growth | Rendered | 55.410 | 60.743 | 242.844 |
| 100,000 | Unchanged targets | None | 0.863 | 1.093 | 0.882 |
| 100,000 | Unchanged targets (six fixed languages) | Rendered | 0.877 | 1.078 | 0.893 |
| 100,000 | Unchanged targets | Rendered | 0.867 | 1.077 | 0.883 |
| 100,000 | Replace one Msg | None | 0.845 | 1.104 | 0.894 |
| 100,000 | Replace one Msg (six fixed languages) | Rendered | 0.882 | 1.041 | 0.886 |
| 100,000 | Replace one Msg | Rendered | 0.854 | 1.035 | 0.883 |
| 100,000 | Reattach the same Msg | None | 0.842 | 0.988 | 0.863 |
| 100,000 | Reattach the same Msg | Rendered | 0.848 | 0.967 | 0.899 |
| 100,000 | Repair ordinary text edits | None | 56.477 | 65.902 | 230.403 |
| 100,000 | Repair ordinary text edits | Rendered | 41.716 | 46.080 | 175.159 |
| 1,048,576 | String-clone formatter | Rendered | 28.587 | 31.737 | 138.977 |
| 1,048,576 | Fluent formatting | Rendered | 148.724 | 159.859 | 717.454 |
| 1,048,576 | Unchanged targets | Rendered | 9.217 | 10.102 | 9.325 |

For ten unchanged bindings, a separate block control measured 1.67 µs
elapsed and 1.75 µs aggregate CPU per refresh. Elapsed is the median of
three process medians; CPU averages the three process means. Each process uses
fifteen blocks of 2,048 refreshes to amortize CPU-clock overhead.

Elapsed time measures each synchronous `PostUpdate` refresh from start to
completion, including waiting for worker tasks. The table takes the median
of three process medians; the percentile column likewise reports the median
of three within-process percentiles. Each process records at least 64 refreshes
for the main workloads and 32 at one million targets. Fast workloads continue
toward a 150 ms observation window, with a 4,096-sample cap that can end it
earlier. Warmup and output validation are outside the timer; measured outliers
are retained. These are descriptive samples, not confidence intervals or
worst-case bounds.

CPU is recorded in a separate trial. It sums all process threads during the
refresh interval, including the caller and workers. Each process contributes
its mean, and the table averages those three process means equally. CPU time is
not added to elapsed time; lower latency can require more aggregate CPU.
CPU-clock overhead is retained in the samples.

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

## Measured checked catalog loading

The following measurements use the same machine and compiler. Each fixture is
one FTL leaf. Generated checked constructors receive a freshly acquired Vec;
input acquisition and catalog destruction are outside the parse timer. They
include UTF-8/syntax/contract validation and construction of the catalog.

Elapsed and CPU values below are **per-catalog batch averages**: batches contain
64 small, 16 medium or four larger catalogs. Median and p95 summarize those batch
averages, with three independent processes and 21 measured batches per process.
They are descriptive observations, rather than individual-request tail bounds.
CPU clocks are read around each batch; their overhead is retained. Allocation
counts come from separate instrumented builds and count requested Rust allocation
bytes, excluding the already acquired input, allocator metadata and fragmentation.
They are neither RSS nor retained catalog size. The catalog also retains the
already acquired source buffer.

| Source bytes | Messages | Median elapsed, ms | 95th-percentile batch-average elapsed, ms | Mean aggregate CPU, ms | Requested allocation bytes |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 4,384 | 32 | 0.022 | 0.029 | 0.024 | 77,084 |
| 35,072 | 256 | 0.187 | 0.256 | 0.205 | 633,260 |
| 271,360 | 1,024 | 0.819 | 0.839 | 0.835 | 2,534,924 |
| 1,050,880 | 256 | 0.402 | 0.423 | 0.411 | 633,260 |

Shared `from_bytes` and embedded sources supply an owned copy to parsing while
retaining their reusable input. Fresh file/callback buffers can transfer their
allocation through the standard constructors. Rendered binding caches are a
separate policy. The number of bindings and the size of a catalog are independent
workload dimensions.

## Language changes through native updates

This headless scenario uses a generated 256-message leaf (35,072 source bytes),
an in-memory async source, checked parsing and real `App::update` publication.
Every Text binding formats the same plain message; each switch verifies every
target after timing. The six background workers are divided into two I/O,
one async-compute and three compute workers. Rendering and text layout are absent.

Each configuration has three processes and nine measured switches per process,
after five warmups. An unpaced runner pumps updates and yields while waiting;
the paced runner waits toward a 16.667 ms update interval. Complete switch
latency includes those waits. Update durations exclude sleeps. Publication is
the last update in a switch, where the new catalog becomes visible and all
bound targets have the expected text. Its p95 is reported separately so that
inexpensive readiness polls do not hide that update's cost. Max is the largest
individual update observed in any switch. All are samples, without a bound on
future frame duration.

| Text bindings | Update pacing | Median complete switch, ms | 95th-percentile publication update, ms | Max individual update, ms | Mean aggregate CPU per switch, ms |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1,024 | Unpaced | 0.483 | 0.246 | 0.375 | 0.699 |
| 1,024 | 16.667 ms | 17.775 | 1.187 | 1.217 | 2.109 |
| 100,000 | Unpaced | 9.076 | 9.851 | 9.976 | 18.211 |
| 100,000 | 16.667 ms | 26.983 | 12.834 | 13.385 | 27.013 |
| 1,048,576 | Unpaced | 65.888 | 63.799 | 66.799 | 167.696 |
| 1,048,576 | 16.667 ms | 79.806 | 68.235 | 69.919 | 168.016 |

Aggregate CPU spans the whole switch, including source acquisition, worker
parsing, publication, polling and target updates. Unpaced polling can consume
CPU while a background task runs. The refresh itself remains synchronous:
large target sets can make the publishing update expensive even when catalog
loading finishes in background tasks. Account for layout, rendering and other
application work when choosing how many visible targets to refresh together.
