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
    let dst_dir = manifest_dir.join("assets").join("_shared");
    let tauri_src = manifest_dir
        .join("..")
        .join("..")
        .join("..")
        .join("apps")
        .join("common")
        .join("ui-assets")
        .join("tauri.js");
    let ui_stepper_src = manifest_dir
        .join("..")
        .join("..")
        .join("..")
        .join("drivers")
        .join("_shared-assets")
        .join("ui-stepper.js");

    println!("cargo:rerun-if-changed={}", tauri_src.display());
    println!("cargo:rerun-if-changed={}", ui_stepper_src.display());

    if let Err(error) = fs::create_dir_all(&dst_dir) {
        panic!(
            "failed to create shared ui-assets dir {}: {error}",
            dst_dir.display()
        );
    }
    copy_shared_asset(&tauri_src, &dst_dir.join("tauri.js"));
    copy_shared_asset(&ui_stepper_src, &dst_dir.join("ui-stepper.js"));
}

fn copy_shared_asset(src: &PathBuf, dst: &PathBuf) {
    if let Err(error) = fs::copy(src, dst) {
        panic!(
            "failed to copy shared ui-asset {} -> {}: {error}",
            src.display(),
            dst.display()
        );
    }
}
