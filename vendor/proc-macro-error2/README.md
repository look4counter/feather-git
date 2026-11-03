# proc-macro-error2 로컬 수정본

이 디렉터리는 crates.io의 proc-macro-error2 2.0.1 소스에 기반한다.
원본 저장소: https://github.com/GnomedDev/proc-macro-error-2

라이선스 원문은 [MIT](LICENSE-MIT), [Apache 2.0](LICENSE-APACHE)에 보존한다.
원본 소스와 검증 자료를 유지하며 배포용 메타데이터와 원본 Markdown 설명 대신
이 한글 문서를 제공한다.

## 로컬 변경

`src/lib.rs`의 `extern crate proc_macro` 선언을 공개로 변경한다.
이미 공개 모듈에서 재노출하는 항목과 가시성을 일치시켜 향후 Rust에서
오류가 되는 비공개 항목 재공개 경고를 해결한다. 의존성 버전과 동작은 유지한다.
