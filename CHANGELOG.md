# Changelog

## 0.1.0 — unreleased

- Extract the updater into a standalone `corestory-tauri-plugin-updater` package.
- Add a caller-supplied verification hook that replaces Minisign when configured.
- Accept `self_update` schema-1 manifests with explicit asset selection.
- Preserve the `updater` runtime and permission namespaces for existing Tauri clients.
- Run JavaScript-triggered installation policies on a blocking worker.
- Fix builds without a TLS backend.
- Keep Minisign as the default verifier and reject verification without a configured policy.

Based on upstream updater 2.14.1. See [upstream release history](docs/upstream-changelog.md) and [source provenance](UPSTREAM.md).
