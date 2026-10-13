fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_os != "android" && target_arch != "wasm32" {
        cc::Build::new()
            .file("native/dsp/kamafeu_dsp.c")
            .include("native/dsp")
            .flag_if_supported("-O3")
            .compile("kamafeu_dsp");
        println!("cargo:rerun-if-changed=native/dsp/kamafeu_dsp.c");
        println!("cargo:rerun-if-changed=native/dsp/kamafeu_dsp.h");
    }
    if target_os == "android" {
        println!("cargo:rustc-link-lib=c++_static");
        println!("cargo:rustc-link-lib=c++abi");
    }
}
