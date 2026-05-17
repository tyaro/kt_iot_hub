fn main() {
    let protoc_path = protoc_bin_vendored::protoc_bin_path().expect("Failed to find protoc");
    std::env::set_var("PROTOC", protoc_path);

    // src-tauri/proto からシンボリックリンクではなく相対パスで参照
    let proto_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../src-tauri/proto");
    let proto_file = format!("{}/driver_runtime.proto", proto_path);

    println!("cargo:rerun-if-changed={}", proto_file);

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile_protos(&[&proto_file], &[proto_path])
        .expect("Failed to compile driver_runtime.proto");
}
