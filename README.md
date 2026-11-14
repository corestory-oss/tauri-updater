# CoreStory Tauri updater

A standalone adaptation of Tauri's updater plugin, with caller-supplied artifact verification and `self_update` schema-1 release manifests. Windows, macOS and Linux installation code comes from upstream.

The Cargo package is `corestory-tauri-plugin-updater`, versioned independently from upstream starting at `0.1.0`. The Rust library remains `tauri_plugin_updater` and the runtime plugin and permission namespace remain `updater`. Use the official `@tauri-apps/plugin-updater` JavaScript package; its commands and returned metadata are unchanged. This repository does not publish JavaScript bindings or depend on the Tauri plugins monorepo.

## Install

```toml
[dependencies]
tauri-plugin-updater = { package = "corestory-tauri-plugin-updater", git = "https://github.com/corestory-oss/tauri-updater" }
```

The dependency key above aliases our package so existing Rust imports continue to work. For reproducible application builds, pin a reviewed commit with `rev` and commit your application's `Cargo.lock`.

This is a distinct Cargo package, not a `[patch.crates-io]` replacement for the upstream package. Replace direct updater dependencies with this package; libraries that depend on upstream must be adapted separately. Both packages retain `links = "tauri-plugin-updater"`, so Cargo rejects linking both implementations into the same dependency graph. No crates.io release has been published.

## Configure verification and asset selection

Register a policy in Rust when constructing the plugin. It also applies to update operations invoked from JavaScript.

```rust,no_run
use tauri_plugin_updater::{Update, VerificationError};

fn updater<R: tauri::Runtime>(
    asset_suffix: String,
    verify: impl Fn(&Update, &[u8]) -> Result<(), VerificationError>
        + Send + Sync + 'static,
) -> tauri::plugin::TauriPlugin<R, tauri_plugin_updater::Config> {
    tauri_plugin_updater::Builder::new()
        .manifest_asset_selector(move |assets| {
            // Choose the exact product, architecture and installer format you distribute.
            // The suffix comes from trusted application configuration.
            let mut matches = assets.iter().enumerate()
                .filter(|(_, asset)| asset.name.ends_with(&asset_suffix));
            let first = matches.next()?.0;
            if matches.next().is_some() { None } else { Some(first) }
        })
        .verifier(verify)
        .build()
}
```

Configure `plugins.updater.endpoints` as usual, pointing at your `manifest.json`. `pubkey` can be omitted when using a custom verifier. Use `UpdaterBuilder::verifier` and `UpdaterBuilder::manifest_asset_selector` for per-instance overrides.

The callback receives the exact downloaded bytes and an `Update` containing version, resolved download URL and `raw_json`. Return `Ok(())` only after your verification succeeds; an error stops installation. A custom verifier **replaces** Minisign and its signed-version policy. No Sigstore dependency or verifier-specific metadata convention is imposed.

With no callback, the existing Minisign verifier is used. With neither a callback nor a public key, verification fails with `MissingVerifier`. Missing Minisign signatures are accepted during discovery, but fail verification when Minisign is the selected policy. There is no automatic fallback from a rejected custom verifier to Minisign or unsigned installation.

`download` verifies on a blocking worker before returning bytes. JavaScript-triggered `install` also runs on a blocking worker. `download_and_install` installs only after that check. `install` verifies its supplied bytes again before extraction or installation, including cached downloads. `verify` and direct `install` call the callback synchronously on their calling thread; invoke them on a blocking worker when appropriate. A callback can run multiple times and should support repeated calls. Retrieve verification materials separately, or use blocking I/O inside the callback; it is not an async callback.

## Manifest format

The accepted schema is the same as [self_update's manifest backend](https://github.com/jaemk/self_update/blob/master/src/backends/manifest.rs):

```json
{
  "schema": 1,
  "releases": [
    {
      "version": "1.2.3",
      "date": "2026-11-14T00:00:00Z",
      "notes_url": "https://example.com/releases/1.2.3",
      "assets": [
        {
          "name": "desktop-1.2.3-aarch64-apple-darwin.app.tar.gz",
          "url": "desktop-1.2.3-aarch64-apple-darwin.app.tar.gz",
          "digest": "sha256:..."
        }
      ]
    }
  ]
}
```

- Only schema `1` is supported. Non-semver versions are skipped. Versions must have no leading `v`.
- The newest semver release is selected regardless of ordering; equal versions keep the first entry. Empty manifests, or manifests with no semver releases, yield no update. Tauri's existing version comparator then decides whether the selected release is an update. No self_update major-version compatibility preference or channel policy is added.
- `manifest_asset_selector` is required for this format. It receives assets with absolute URLs and returns an index, or `None` to reject selection. Missing matches and out-of-range indices are errors. Selection happens before the version comparator; the newest release must contain an appropriate asset even when it is not newer than the installed application.
- Relative asset URLs resolve against the final manifest response URL, including redirects, using standard URL joining. This deliberately handles `..` and query strings more robustly than self_update's simple string concatenation.
- `date`, `notes_url` and asset `digest` are optional. Valid RFC3339 dates populate Tauri's typed date; other dates remain available as metadata. `notes_url` is retained without fetching release notes.
- Unknown fields are retained for verification policies. `digest` is metadata only: your verifier decides whether and how to check it. A digest in unauthenticated metadata alone does not authenticate a publisher.
- Assets must contain packages Tauri can install, such as a macOS app archive, Windows installer or Linux package/AppImage. Sharing a schema does not make a CLI binary archive installable as a desktop app.

Existing Tauri static and dynamic manifests remain supported.

For shared manifests, `Update::raw_json` is a normalized, self-contained dynamic Tauri response. It includes `asset` (the selected asset), `assets` (all resolved assets), `release` (the original release including extension fields), `manifest_url`, and `notes_url`. Save this value and restore it with `Updater::restore_update`; no network request or original base URL is needed. Restored updates use the application's current verifier, never a policy from cached metadata.

Artifact verification authenticates whatever claims your policy checks. Version, product, platform and channel claims in manifest or cached metadata remain untrusted unless your verifier binds them to the artifact. Minisign restores retain upstream's signed-version requirement.

## Development

```sh
cargo fmt --all -- --check
cargo test --lib
cargo test --doc
cargo clippy --all-targets -- -D warnings
```

The upstream minimum Rust version is 1.90. The [CI template](docs/github-actions-ci.yml) checks macOS, Linux and Windows. To enable it, copy it to `.github/workflows/ci.yml` using credentials with GitHub workflow permission. Local validation records are in [UPSTREAM.md](UPSTREAM.md). The unit tests do not substitute for exercising real app replacement on each shipping platform.

## Origin and license

Extracted from `tauri-apps/plugins-workspace/plugins/updater`, with upstream copyright and Apache-2.0 / MIT license files preserved. [UPSTREAM.md](UPSTREAM.md) records the exact baseline and adaptation scope.
