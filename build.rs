fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS");
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV");
    if target_os.as_deref() == Ok("windows") && target_env.as_deref() == Ok("msvc") {
        // Keep executable exports, but do not generate unused import libraries
        // or export object files whose progress messages become rustc warnings.
        println!("cargo::rustc-link-arg=/NOIMPLIB");
        println!("cargo::rustc-link-arg=/NOEXP");
    }
}
