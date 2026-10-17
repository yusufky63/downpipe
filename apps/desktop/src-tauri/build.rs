fn main() {
    skip_sidecar_copy();
    tauri_build::build();
}

/// tauri-build copies every `bundle.externalBin` into `target/<profile>/`,
/// which for the native host is the file Cargo already builds there from
/// this workspace. The copy would only swap a fresh build for whatever
/// `binaries/` last held (the native host's end-to-end tests run that file),
/// and it fails outright before `pnpm sidecar` has run, which breaks plain
/// `cargo check --workspace`. The bundler reads `binaries/` itself, so the
/// installers still get the sidecar.
fn skip_sidecar_copy() {
    const VAR: &str = "TAURI_CONFIG";
    // tauri-build merges this JSON over tauri.conf.json; the CLI sets it for
    // `--config`, so keep what is there.
    let mut patch = std::env::var(VAR)
        .ok()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
        .filter(serde_json::Value::is_object)
        .unwrap_or_else(|| serde_json::json!({}));
    // A JSON merge patch removes keys set to null.
    patch["bundle"]["externalBin"] = serde_json::Value::Null;
    // SAFETY: the build script is still single-threaded here.
    unsafe { std::env::set_var(VAR, patch.to_string()) };
}
