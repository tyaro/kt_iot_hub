fn main() {
    let protoc_path = protoc_bin_vendored::protoc_bin_path().expect("Failed to find protoc");
    std::env::set_var("PROTOC", protoc_path);

    println!("cargo:rerun-if-changed=proto/tag_registration.proto");

    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(&["proto/tag_registration.proto"], &["proto"])
        .expect("Failed to compile gRPC proto");

    tauri_build::build()
}
