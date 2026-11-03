# 조사 기록

## 경고 목록

- GPUI `taffy.rs:312`: length와 fr의 리터럴이 f64에서 f32로 암묵 fallback되는 경고 2개.
- proc-macro-error2 2.0.1: 비공개 extern crate proc_macro를 재공개하여 향후 오류가 되는 E0365.
- MSVC LINK: libgit2 export가 있는 실행 파일을 링크하면서 .lib와 .exp를 생성한다는 안내가 rustc의 linker_messages 경고로 표시됨.

## 결정과 근거

- 부동소수점 리터럴을 명시적 f32로 작성한다. 값과 실행 의미는 바꾸지 않는다.
- proc-macro-error2의 외부 크레이트 선언을 공개한다. 이미 공개 재노출하는 항목과 가시성을 일치시킨다. 현재 로컬 캐시에 다른 버전이 없으므로 버전 갱신 대신 재현 가능한 로컬 패치를 사용한다.
- 연구 에이전트가 로컬 link.exe 도움말에서 /NOIMPLIB 지원을 확인했다. 불필요한 import library 생성을 중지하고 exe export는 전후 이름 비교로 검증한다.
- 실제 export 원인은 libgit2이며 GPU 선택 심볼이 아니었다. 현재 실행 파일에는 약 950개의 git_* export가 있다.

## 제외한 대안

- 모든 warning 또는 linker_messages 차단: 실제 문제를 가릴 수 있어 제외.
- 사용자 Cargo 캐시 수정: 다른 환경에서 재현되지 않아 제외.
- 무관한 의존성 전체 갱신: 기존 입력 수정과 버전 안정성에 불필요한 영향을 주므로 제외.

## 참고

- [Rust 링커 진단 추적 이슈](https://github.com/rust-lang/rust/issues/136096)
- [Microsoft import library 설명](https://learn.microsoft.com/en-us/cpp/build/reference/working-with-import-libraries-and-export-files)
- `cargo report future-incompatibilities --id 2`와 로컬 MSVC `/ ?` 도움말을 근거로 사용했다.

## 링커 소규모 검증 결과

실제 MSVC 14.51.36231과 Rust의 작은 export 예제로 검증했다. /NOIMPLIB만 적용하면 .exp 생성 안내가 남았다. /NOIMPLIB와 /NOEXP를 함께 적용하면 안내·경고가 0건이며 export 항목은 보존되었다. 두 옵션을 함께 적용한다. 원래 debug와 release의 export 이름 목록은 각각 950개를 `.build/warnings-*-exports-before.txt`에 기록했다.
