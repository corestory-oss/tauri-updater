# Upstream baseline and maintenance

- Source: https://github.com/tauri-apps/plugins-workspace/tree/9aa4f717152cdeceed4bada9c849ad68a9dc402c/plugins/updater
- Branch at extraction: `v2`
- Upstream crate version: `2.14.1`
- Extraction date: November 14, 2026
- Original directory: `plugins/updater/`
- Licenses: Apache-2.0 OR MIT; original license files and source attribution retained.

The first commit is an unchanged import of that plugin directory. The adaptation commit makes Cargo dependencies explicit, preserves the Rust library and runtime plugin identities for consumer compatibility, adds a generic verifier and a schema-1 manifest adapter, and documents the changed verification boundary. The adapted Cargo package is `corestory-tauri-plugin-updater`, versioned independently at `0.1.0`. The original upstream version is provenance, not our release version. The library remains `tauri_plugin_updater`; `links` and generated ACL metadata deliberately retain the updater identity for JS and capability compatibility. `build.rs` calls Tauri's public build helpers with that explicit identity rather than deriving it from our Cargo package name.

The official npm client remains usable. Upstream frontend source/build configuration and monorepo-only test applications were removed from the maintained tree; the generated `api-iife.js` is retained for Tauri's global API support. Upstream Rust unit tests remain, with additional manifest and verification-path tests. This repository does not include Tauri core, other plugins, or application-specific verification code.

## Porting upstream changes

Compare the plugin directory at the baseline commit above against the desired upstream revision. Port relevant fixes, especially installation, rollback, transport and verification changes, then rerun local checks and, once enabled, the platform CI matrix. Preserve the invariant that a custom verification failure stops every installation path. Update this file with the new source baseline once a rebase or port is complete.

The schema adapter follows self_update's schema-1 fields, not its entire release-selection or installation algorithm. Asset selection is explicit; metadata normalization and standard URL joining are intentional differences documented in the README.

## Local validation

Validated on macOS arm64 with Rust 1.99.1:

- 34 Rust unit tests, including all 25 inherited tests.
- Four compiled Rust documentation examples.
- Clippy across all targets and all crate features with warnings denied; this also checks the custom-verifier example.
- Rust formatting.
- `cargo package` builds the packaged crate without publishing.
- A consumer build checks dependency aliasing, the `updater` permission namespace, global API metadata, and command pruning.
- Feature checks with no TLS backend and with all features.
- `cargo +1.90 check --lib` against the declared minimum Rust version.

Tests cover missing verifiers, replacement of Minisign, custom-policy rejection before direct installation, successful verified downloads, rejection through combined download/install, cached metadata restoration, schema errors, URL resolution, explicit asset selection, and propagation of the plugin policy to the JS installation worker. The HTTP tests use local loopback fixtures.

Real desktop application replacement has not been smoke-tested locally. The CI template in `docs/github-actions-ci.yml` defines Linux, macOS and Windows checks. The cross-platform workflow has not yet been run. Copy it to `.github/workflows/ci.yml` with appropriately scoped credentials to enable it.
