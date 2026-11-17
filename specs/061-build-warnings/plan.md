# 구현 계획

## 기술과 분석
Rust 앱의 backend::module 매크로가 래퍼를 생성한다. commit_files 래퍼와 구현, diff 및 remember 래퍼는 테스트에서만 호출된다. diff와 remember 구현은 내부 일반 경로에서도 쓰이므로 유지한다. 매크로가 함수별 속성을 전달하도록 확장하고 해당 래퍼만 cfg(test)로 제한한다. GraphBuilder::append가 일반 그래프 경로이므로 일괄 생성 보조 함수는 테스트 전용으로 둔다. Diff.title은 읽는 곳이 없고 실제 제목은 ViewerTarget에서 제공된다. open_keys에는 호출부가 없으므로 해당 메서드만 제거하며 다른 단축키 설정 기능과 테스트는 유지한다.

## 영향 및 설계
src/backend.rs, repository.rs, graph.rs, diff_model.rs, diff_git.rs, diff_viewer.rs 및 vendor/gpui/src/taffy.rs를 최소 변경한다. proc-macro-error2 2.0.1은 Cargo의 기존 로컬 패치 방식으로 vendor에 두고 proc_macro의 extern crate 선언만 public으로 만든다. 외부 캐시를 수정하지 않는다. 라이선스를 복사하고 패치 설명은 한글로 작성한다. 주요 의존성 교체나 추가는 없다.

## 불변 조건
헌법은 미작성 템플릿이다. UI·Git 데이터 흐름과 기존 테스트를 유지하며 전역 경고 억제를 도입하지 않는다. 일반 빌드에서 제거되는 함수는 앱 내부 테스트 전용 API이다.

## 검증
cargo fmt 검사, cargo check 및 clippy, 전체 테스트, 별도 실행 파일 빌드와 새 future incompatibility 보고서를 확인한다. 의존성 로컬 패치로 재빌드가 발생할 수 있다. 앱 실행은 사용자 프로세스를 종료하지 않는다.

Diff.title 생성에만 쓰였던 FileChange::stage_status도 함께 정리한다. 다른 상태 표시 경로에는 영향이 없다.

정적 분석에서 확인한 src/ui.rs의 collapsible_if 한 건도 같은 조건의 let 체인으로 정리한다. 로컬 패치의 원본 테스트 파일도 보존한다.

## 추가 계획: Windows MSVC 링커 안내

### 분석과 결정

Rust 1.98.0에서 cargo build --release --offline으로 안내 경고를 재현했다. Cargo.toml은 실행 파일 패키지이며 루트 build.rs와 .cargo 설정은 없다. MSVC는 내보내기가 있는 실행 파일에도 임포트 라이브러리를 생성할 수 있다. 기존 일반 경고 정리는 이 메시지를 예외로 기록했으나 이번 요구사항은 이를 해소한다.

루트 build.rs에서 CARGO_CFG_TARGET_OS와 CARGO_CFG_TARGET_ENV가 windows·msvc인 경우에만 cargo:rustc-link-arg-bin=feather-git=/NOIMPLIB을 출력한다. 앱 실행 파일에만 적용하며 Cargo.toml 수정이나 새 의존성은 필요하지 않다. 임포트 라이브러리를 소비하는 대상은 없다. 실행 파일 내보내기와 GUI 서브시스템은 실제 PE 헤더로 전후 대조한다.

/NOLOGO는 시작 배너용이며 대상 메시지 해결을 보장하지 않는다. linker_messages 전체 억제는 실제 진단을 가리므로 제외한다. 전역 rustflags는 의존성과 테스트까지 영향을 주므로 제외한다.

### 검증과 헌장 점검

추가 전후 PE 내보내기 이름과 서브시스템을 대조한다. 새 릴리스 링크와 개발·테스트 타깃 검사, 빌드 스크립트 형식 및 diff 검사를 실행한다. 단순 조건문을 복제하는 테스트는 추가하지 않는다. 헌장은 미작성 템플릿이므로 규범 검사를 생략한다. 기존 아키텍처·API·실행 흐름·의존성을 유지하므로 설계 전후 충돌은 없다. 확장 훅 파일은 없다.

### 검증에 따른 계획 보완

현재 MSVC에서는 /NOIMPLIB만으로 .exp 안내가 남는다. 동일한 실행 파일 범위에 /NOEXP도 전달하여 두 보조 산출물의 생성을 막는다. PE 내보내기와 GUI 서브시스템 전후 대조는 그대로 수행한다. T006 구현 및 T007 검증에 두 옵션을 반영한다. 재분석 결과 요구사항·계획·작업 사이 누락과 충돌은 없다.
