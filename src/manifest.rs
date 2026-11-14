// Copyright 2026 CoreStory
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Adapter for self_update's schema-1 static manifests.

use std::{collections::BTreeMap, sync::Arc};

use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{Error, RemoteRelease, Result};

/// A named release artifact from a self_update manifest.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestAsset {
    /// Artifact filename. The caller decides which names are appropriate for this application.
    pub name: String,
    /// Download URL, resolved against the final manifest response URL before selection.
    pub url: Url,
    /// Optional `algorithm:hex` digest. Passed to the verifier; not verified automatically.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
    /// Additional fields, available to custom verification policies.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Returns the chosen asset index, or None when no asset is appropriate.
/// An out-of-range index is an error. Selection is independent of verification.
pub type ManifestAssetSelector = Arc<dyn Fn(&[ManifestAsset]) -> Option<usize> + Send + Sync>;

#[derive(Deserialize)]
struct Manifest {
    schema: u64,
    #[serde(default)]
    releases: Vec<Release>,
}

#[derive(Deserialize, Serialize)]
struct Release {
    version: String,
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    assets: Vec<Asset>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

#[derive(Deserialize, Serialize)]
struct Asset {
    name: String,
    url: String,
    #[serde(default)]
    digest: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

pub(crate) fn parse_manifest(
    body: Value,
    manifest_url: &Url,
    selector: Option<&ManifestAssetSelector>,
) -> Result<Option<(RemoteRelease, Value)>> {
    let manifest: Manifest = serde_json::from_value(body)?;
    if manifest.schema != 1 {
        return Err(Error::Manifest(format!(
            "unsupported schema {}; expected 1",
            manifest.schema
        )));
    }
    // Skip non-semver releases as self_update does. The newest semver wins regardless of order;
    // a tie keeps the first entry. No implicit channel or compatibility policy is added.
    let mut latest = None;
    for release in manifest.releases {
        let Ok(version) = Version::parse(&release.version) else {
            continue;
        };
        if latest
            .as_ref()
            .is_none_or(|(previous, _)| version > *previous)
        {
            latest = Some((version, release));
        }
    }
    let Some((version, release)) = latest else {
        return Ok(None);
    };
    let original_release = serde_json::to_value(&release)?;
    let selector = selector.ok_or_else(|| {
        Error::Manifest("self_update manifests require a manifest_asset_selector".into())
    })?;
    let assets: Vec<_> = release
        .assets
        .into_iter()
        .map(|asset| {
            Ok(ManifestAsset {
                name: asset.name,
                url: manifest_url.join(&asset.url)?,
                digest: asset.digest,
                extra: asset.extra,
            })
        })
        .collect::<Result<_>>()?;
    let index =
        selector(&assets).ok_or_else(|| Error::Manifest("no matching release asset".into()))?;
    let asset = assets
        .get(index)
        .ok_or_else(|| Error::Manifest("asset selector returned an invalid index".into()))?;
    // Normalize to the existing dynamic Tauri shape so raw_json is self-contained and can be
    // restored offline without guessing the original manifest base URL. Preserve extension fields.
    let mut metadata = serde_json::json!({
        "release": original_release,
        "notes_url": release.extra.get("notes_url"),
    });
    metadata["version"] = Value::String(version.to_string());
    metadata["url"] = serde_json::to_value(&asset.url)?;
    metadata["signature"] = Value::String(String::new());
    if let Some(date) = release.date {
        // self_update treats dates as opaque strings. Tauri exposes parsed RFC3339 dates;
        // retain the original date even when it cannot populate the typed date field.
        metadata["date"] = Value::String(date.clone());
        if time::OffsetDateTime::parse(&date, &time::format_description::well_known::Rfc3339)
            .is_ok()
        {
            metadata["pub_date"] = Value::String(date);
        }
    }
    metadata["asset"] = serde_json::to_value(asset)?;
    metadata["assets"] = serde_json::to_value(&assets)?;
    metadata["manifest_url"] = serde_json::to_value(manifest_url)?;
    let remote = serde_json::from_value(metadata.clone())?;
    Ok(Some((remote, metadata)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base() -> Url {
        "https://updates.example/releases/manifest.json"
            .parse()
            .unwrap()
    }
    fn selector() -> ManifestAssetSelector {
        Arc::new(|assets| assets.iter().position(|a| a.name.ends_with(".app.tar.gz")))
    }
    fn manifest() -> Value {
        json!({"schema":1,"releases":[
            {"version":"1.0.0","assets":[]},
            {"version":"3.0.0","date":"2026-11-14T00:00:00Z","notes_url":"https://example/notes",
             "assets":[
                {"name":"cli-aarch64-apple-darwin.tar.gz","url":"cli.tar.gz"},
                {"name":"desktop.app.tar.gz","url":"desktop.tar.gz","digest":"sha256:abc","bundle":"desktop.bundle"}
             ]},
            {"version":"nightly","assets":[]},
            {"version":"2.0.0","assets":[]}
        ]})
    }

    #[test]
    fn selects_newest_and_resolves_urls_preserving_verifier_metadata() {
        let (release, metadata) = parse_manifest(manifest(), &base(), Some(&selector()))
            .unwrap()
            .unwrap();
        assert_eq!(release.version, Version::new(3, 0, 0));
        assert_eq!(
            release.download_url("any").unwrap().as_str(),
            "https://updates.example/releases/desktop.tar.gz"
        );
        assert_eq!(metadata["asset"]["digest"], "sha256:abc");
        assert_eq!(metadata["asset"]["bundle"], "desktop.bundle");
        assert_eq!(metadata["notes_url"], "https://example/notes");
        assert!(release.pub_date.is_some());
        // The normalized metadata restores through the existing Tauri parser.
        let restored: RemoteRelease = serde_json::from_value(metadata).unwrap();
        assert_eq!(restored.version, release.version);
    }

    #[test]
    fn requires_explicit_valid_asset_selection() {
        assert!(parse_manifest(manifest(), &base(), None).is_err());
        for index in [None, Some(99)] {
            let selector: ManifestAssetSelector = Arc::new(move |_| index);
            assert!(parse_manifest(manifest(), &base(), Some(&selector)).is_err());
        }
    }

    #[test]
    fn rejects_bad_schema_or_required_fields() {
        for value in [
            json!({"schema":2}),
            json!({"releases":[]}),
            json!({"schema":1,"releases":[{"version":"2.0.0","assets":[{"name":"app"}]}]}),
        ] {
            assert!(parse_manifest(value, &base(), Some(&selector())).is_err());
        }
    }

    #[test]
    fn empty_or_non_semver_manifests_have_no_update() {
        for releases in [json!([]), json!([{"version":"nightly"}])] {
            assert!(
                parse_manifest(json!({"schema":1,"releases":releases}), &base(), None)
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn absolute_urls_and_opaque_dates_are_supported() {
        let value = json!({"schema":1,"releases":[{"version":"2.0.0","date":"yesterday","assets":[{"name":"app.app.tar.gz","url":"https://cdn.example/app.tgz"}]}]});
        let (release, metadata) = parse_manifest(value, &base(), Some(&selector()))
            .unwrap()
            .unwrap();
        assert!(release.pub_date.is_none());
        assert_eq!(metadata["date"], "yesterday");
        assert_eq!(metadata["url"], "https://cdn.example/app.tgz");
    }
}
