fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "android" && target_os != "linux" {
        panic!(
            "chimerad must be built for android or linux only, got: {}",
            target_os
        );
    }
}
