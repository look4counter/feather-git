# 작업 목록: 빌드 경고 정리

**입력**: [명세](spec.md), [계획](plan.md), [조사](research.md)

## 1단계: 준비

- [x] T001 현재 경고와 `target/debug/feather-git.exe`, `target/release/feather-git.exe`의 export 기준 목록을 `.build/warnings-*-exports-before.txt`에 기록한다.

## 2단계: 기반

- [x] T002 `vendor/proc-macro-error2/`에 기존 의존성과 라이선스를 보존하고 `Cargo.toml`, `Cargo.lock`에 로컬 패치를 연결한다. (FR-005)

## 3단계: 사용자 이야기 1 — 경고 없는 빌드

**독립 검증**: 개발·배포 빌드의 경고가 없고 기존 입력 회귀 및 export 비교가 통과한다.

- [x] T003 [US1] `vendor/gpui/src/taffy.rs`에 명시적 f32 리터럴을 적용하고 `vendor/proc-macro-error2/src/lib.rs`의 proc_macro 선언 가시성을 수정한다. (FR-001~002)
- [x] T004 [US1] `build.rs`에서 Windows MSVC 대상에만 /NOIMPLIB와 /NOEXP를 전달하여 불필요한 .lib/.exp 생성 안내를 제거한다. (FR-003)
- [x] T005 [US1] 개발·배포 빌드, `src/commit_editor.rs` 입력 회귀 15개, 실행 파일 export 비교를 실행하여 `.build/`에 결과를 보관한다. (FR-004, SC-001~003)

## 4단계: 정리

- [x] T006 `vendor/gpui/README.md`, `vendor/proc-macro-error2/README.md`에 출처와 변경을 기록하고 `specs/008-clean-build-warnings/quickstart.md`에 검증 결과를 정리한다.

## 의존성과 실행 전략

T001 → T002 → T003 → T004 → T005 → T006 순서로 진행한다. 최소 제공 범위는 사용자 이야기 1 전체다. 각 빌드가 같은 Cargo 산출물에 접근하므로 순차 실행한다. 독립적인 문서 확인과 export 기준 목록 조사는 병렬로 가능하다. 별도 단위 검증을 추가하기보다 실제 재컴파일과 기존 입력 회귀를 사용한다.
