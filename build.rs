fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "android" && target_os != "linux" && cfg!(not(debug_assertions)) {
        panic!(
            "chimerad must be built for android or linux only, got: {}",
            target_os
        );
    }
    if target_os != "android" && target_os != "linux" {
        println!("cargo:warning=chimerad is designed for android/linux, allowing build on {} for development/testing", target_os);
    }
}
