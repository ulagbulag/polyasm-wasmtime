use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    set_commit_info_for_rustc();

    println!("cargo:rustc-check-cfg=cfg(asan)");
    match env::var("CARGO_CFG_SANITIZE") {
        Ok(s) if s == "address" => {
            println!("cargo:rustc-cfg=asan");
        }
        _ => {}
    }
}

fn set_commit_info_for_rustc() {
    println!(
        "cargo:rustc-env=WASMTIME_VERSION_INFO={}",
        env!("CARGO_PKG_VERSION")
    );
}
