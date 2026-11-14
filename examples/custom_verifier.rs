// Copyright 2026 CoreStory
// SPDX-License-Identifier: Apache-2.0 OR MIT

use tauri_plugin_updater::{Config, Update, VerificationError};

// A policy is injected by the application. This example intentionally supplies no permissive
// placeholder verifier: production cryptography and proof retrieval belong to the caller.
fn plugin<R: tauri::Runtime>(
    suffix: String,
    verifier: impl Fn(&Update, &[u8]) -> Result<(), VerificationError> + Send + Sync + 'static,
) -> tauri::plugin::TauriPlugin<R, Config> {
    tauri_plugin_updater::Builder::new()
        .manifest_asset_selector(move |assets| {
            let mut matches = assets
                .iter()
                .enumerate()
                .filter(|(_, a)| a.name.ends_with(&suffix));
            let first = matches.next()?.0;
            if matches.next().is_some() {
                None
            } else {
                Some(first)
            }
        })
        .verifier(verifier)
        .build()
}

fn main() {
    // Reference the factory so `cargo check --examples` validates the public API.
    let _ = plugin::<tauri::Wry>("desktop.app.tar.gz".into(), |_, _| {
        Err(std::io::Error::other("supply an application verification policy").into())
    });
}
