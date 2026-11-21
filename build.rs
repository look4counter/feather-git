fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=design/favicon.rc");
        println!("cargo:rerun-if-changed=design/favicon.ico");
        embed_resource::compile_for("design/favicon.rc", ["feather-git"], embed_resource::NONE)
            .manifest_required()
            .expect("Windows 앱 아이콘 리소스를 빌드하지 못했습니다.");

        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            // 실행 파일의 내보내기는 유지하고 불필요한 링크용 보조 파일만 생략한다.
            println!("cargo:rustc-link-arg-bin=feather-git=/NOIMPLIB");
            println!("cargo:rustc-link-arg-bin=feather-git=/NOEXP");
        }
    }
}
