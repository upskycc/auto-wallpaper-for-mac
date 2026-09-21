fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() == Some("macos") {
        cc::Build::new().file("src/display_wait.c").compile("display_wait");
        println!("cargo:rustc-link-lib=framework=IOKit");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rerun-if-changed=src/display_wait.c");
    }
}
