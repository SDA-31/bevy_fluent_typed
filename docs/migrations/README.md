# Migration Guide

These guides upgrade an existing application to **bevy_fluent_typed 0.4.0**.
The generator stays at **fluent_typed_codegen 0.2.2**. You do not need to install
intermediate runtime releases.

Choose the version you are upgrading from:

| Your current version | Start here |
| --- | --- |
| 0.3.1 | [Explicit target bindings, Msg and cache policies](migration-0.3.1.md) |
| 0.3.0 | [Native catalog registration and startup](migration-0.3.0.md) |
| 0.2.2 | [Automatic loading, language switching and backend selection](migration-0.2.2.md) |
| 0.2.1 | [Catalog paths, selective embedding and typed errors](migration-0.2.1.md) |
| 0.2.0 | [Native resource waiting](migration-0.2.0.md) |
| 0.1.3 | [Facade, sources, readiness and handwritten providers](migration-0.1.3.md) |

For versions older than 0.3.1, also apply the
[0.4.0 binding changes](migration-0.3.1.md). The older-version guides link to the
loading, startup and build-configuration steps relevant to those applications.

After upgrading, check startup, a language switch, changed message arguments
and target removal/reinsertion in your application. Check visible text and
accessibility labels independently when using both.

[Quickstart](../../README.md#setup) · [Loading guide](../../GUIDE.md) ·
[Changelog](../../CHANGELOG.md)
