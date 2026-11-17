fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // 실행 파일의 내보내기는 유지하고 불필요한 링크용 보조 파일만 생략한다.
        println!("cargo:rustc-link-arg-bin=feather-git=/NOIMPLIB");
        println!("cargo:rustc-link-arg-bin=feather-git=/NOEXP");
    }
}
