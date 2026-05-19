fn main() {
    let protoc_path = protoc_bin_vendored::protoc_bin_path().expect("Failed to find protoc");
    std::env::set_var("PROTOC", protoc_path);

    println!("cargo:rerun-if-changed=proto/tag_registration.proto");
    println!("cargo:rerun-if-changed=proto/driver_runtime.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &["proto/tag_registration.proto", "proto/driver_runtime.proto"],
            &["proto"],
        )
        .expect("Failed to compile gRPC proto");

    ensure_bundle_resource_placeholders();
    tauri_build::build()
}

fn ensure_bundle_resource_placeholders() {
    let resource_paths = [
        "../../ops/driver-ui/postgres/registration-ui.exe",
        "../../ops/driver-ui/postgres/driver-postgres.exe",
        "../../ops/driver-ui/joywatcher/registration-ui.exe",
        "../../ops/driver-ui/joywatcher/driver-joywatcher.exe",
        "../../ops/driver-ui/joywatcher/joywatcher-bridge-x86.exe",
    ];

    for relative in resource_paths {
        println!("cargo:rerun-if-changed={}", relative);
        let path = std::path::Path::new(relative);
        if path.exists() {
            continue;
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create driver-ui placeholder dir");
        }

        std::fs::write(path, b"placeholder").expect("Failed to create resource placeholder");
    }
}
