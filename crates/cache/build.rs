fn main() {
    println!("cargo:rustc-env=GIT_REV={}", env!("CARGO_PKG_VERSION"));
}
