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
