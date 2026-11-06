fn main() {
    println!("cargo:rustc-env=COMPILER_VERSION={}", env!("CARGO_PKG_VERSION"));
    println!("cargo:rustc-env=USE_MTIME=false");
}
