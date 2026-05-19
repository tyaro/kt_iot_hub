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
        "../../ops/driver-ui/postgres/driver-manifest.json",
        "../../ops/driver-ui/joywatcher/registration-ui.exe",
        "../../ops/driver-ui/joywatcher/driver-joywatcher.exe",
        "../../ops/driver-ui/joywatcher/joywatcher-bridge-x86.exe",
        "../../ops/driver-ui/joywatcher/driver-manifest.json",
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

        let placeholder = manifest_placeholder(path)
            .map(|content| content.as_bytes().to_vec())
            .unwrap_or_else(|| b"placeholder".to_vec());

        std::fs::write(path, placeholder).expect("Failed to create resource placeholder");
    }
}

fn manifest_placeholder(path: &std::path::Path) -> Option<&'static str> {
    let file_name = path.file_name()?.to_str()?;
    if file_name != "driver-manifest.json" {
        return None;
    }

    let driver_type = path.parent()?.file_name()?.to_str()?;
    match driver_type {
        "postgres" => Some(
            r#"{
  "manifestVersion": 1,
  "driverType": "postgres",
  "displayName": "PostgreSQL 接続",
  "registrationUi": "registration-ui.exe",
  "runtime": "driver-postgres.exe",
  "protocol": {
    "driverUiRequestVersion": "1",
    "driverUiResponseVersion": "1"
  },
  "capabilities": ["connectionTest", "schemaProvided"]
}"#,
        ),
        "joywatcher" => Some(
            r#"{
  "manifestVersion": 1,
  "driverType": "joywatcher",
  "displayName": "JoyWatcher ジョイスティック",
  "registrationUi": "registration-ui.exe",
  "runtime": "driver-joywatcher.exe",
  "protocol": {
    "driverUiRequestVersion": "1",
    "driverUiResponseVersion": "1"
  },
  "capabilities": ["connectionTest", "schemaProvided", "multipleInstances"]
}"#,
        ),
        _ => None,
    }
}
