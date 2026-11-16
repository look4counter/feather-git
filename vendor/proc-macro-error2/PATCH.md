# 로컬 호환성 패치

원본: proc-macro-error2 2.0.1 (https://github.com/GnomedDev/proc-macro-error-2).
원본 라이선스와 소스 저작권을 유지한다. README 원문은 README.upstream.txt에 보존한다.

수정: src/lib.rs의 extern crate proc_macro를 pub extern crate로 변경한다. __private 모듈의 기존 pub use와 가시성을 일치시켜 향후 Rust 호환성 경고 E0365를 해결한다. 버전과 매크로 동작은 유지한다. 패키지 README 경로만 보존 파일명에 맞췄다.
