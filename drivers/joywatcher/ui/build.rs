use std::fs;
use std::path::PathBuf;

fn main() {
    // frontendDist (./assets) は WebView に埋め込まれるため、
    // その外にある apps/common/ui-assets/ を release ビルドで import できない。
    // ビルド時に assets/_shared/ へコピーして同一オリジンで参照可能にする。
    sync_shared_ui_assets();

    tauri_build::build()
}

fn sync_shared_ui_assets() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir
        .join("..")
        .join("..")
        .join("common")
        .join("ui-assets")
        .join("tauri.js");
    let dst_dir = manifest_dir.join("assets").join("_shared");
    let dst = dst_dir.join("tauri.js");

    println!("cargo:rerun-if-changed={}", src.display());

    if let Err(error) = fs::create_dir_all(&dst_dir) {
        panic!(
            "failed to create shared ui-assets dir {}: {error}",
            dst_dir.display()
        );
    }
    if let Err(error) = fs::copy(&src, &dst) {
        panic!(
            "failed to copy shared ui-asset {} -> {}: {error}",
            src.display(),
            dst.display()
        );
    }
}
