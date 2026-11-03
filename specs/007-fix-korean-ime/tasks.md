# 작업 목록: 한글 입력 보존

**입력**: [명세](spec.md), [계획](plan.md), [조사](research.md)

## 1단계: 준비

- [x] T001 GPUI 원본과 라이선스를 `vendor/gpui/`에 보존하고 로컬 변경 내역을 한글로 기록한다.

## 2단계: 기반

- [x] T002 `Cargo.toml`, `Cargo.lock`을 로컬 GPUI 패치와 연결한다.

## 3단계: 사용자 이야기 1 — 한글과 영문 연속 입력

**목표**: `쿼리ID` 입력에서 글자 손실을 제거한다.
**독립 검증**: 조합 종료 전후 확정 순서, 동시 확정·새 조합, 취소, 이모지·여러 줄·후속 편집 시퀀스를 실제 버퍼로 검증한다.

- [x] T003 [US1] `src/commit_editor.rs`에 위 입력 시퀀스의 회귀 검증을 추가하고 기존 실패를 확인한다. (FR-001~004, SC-001~002)
- [x] T004 [US1] `vendor/gpui/src/platform/windows/ime.rs`에 종료·확정·취소 상태 전이를 구현하고 `events.rs`, `window.rs`의 실제 메시지 처리에 연결한다. (FR-001~004)
- [x] T005 [US1] `src/commit_editor.rs`의 회귀 검증 및 기존 편집기 검증과 애플리케이션 빌드를 실행한다. (SC-001~002)

## 4단계: 검증 정리

- [x] T006 `specs/007-fix-korean-ime/quickstart.md`에 실행한 검증 결과와 수동 입력 확인 여부를 기록하고 명세·계획·구현 수렴을 확인한다.

## 의존성과 실행 전략

T001 → T002 → T003 → T004 → T005 → T006 순서로 수행한다. 최소 제공 범위는 사용자 이야기 1 전체다. 같은 파일과 컴파일 의존성이 있어 구현 작업을 병렬로 수행하지 않는다. 문서 검토는 코드 검증과 독립적으로 가능하다.

## 5단계: 배포 실행 파일 수렴

기존 release 파일은 2026-11-01 빌드이며 의존성 목록에 로컬 GPUI 수정본이 없었다. 사용 기록의 실행 경로도 release를 가리켰다. 소스 검증 외에 실제 사용하는 빌드 산출물의 갱신이 누락되었다.

- [x] T007 `cargo build --offline --release --bin feather-git`으로 `target/release/feather-git.exe`를 다시 빌드한다. (FR-001, 부분 충족)
- [x] T008 `target/release/feather-git.d`의 로컬 GPUI 의존성과 실행 파일 갱신을 확인하고 `specs/007-fix-korean-ime/quickstart.md`에 실제 입력 재현의 승인 제한을 기록한다. (SC-001, 부분 충족)

## 6단계: 실제 입력 재조사

- [x] T009 [US1] `vendor/gpui/src/platform/windows/ime_trace.rs`, `events.rs`에 환경변수로 활성화하는 본문 없는 로컬 입력 진단을 추가한다. (FR-005)
- [x] T010 [US1] `cargo run`으로 실제 `쿼리I` 입력을 재현하여 `.build/ime-trace.log`의 메시지 순서와 조합 상태를 확인한다. (FR-001, SC-003)
- [x] T011 [US1] 관측한 순서에 맞춰 `vendor/gpui/src/platform/windows/ime.rs` 및 필요 시 `events.rs`를 수정하고 `src/commit_editor.rs` 회귀 검증에 추가한다. (FR-001~004)
- [x] T012 [US1] 실제 입력 재확인과 편집기 검증 후 `specs/007-fix-korean-ime/quickstart.md`에 결과를 기록한다. (SC-001~003)

T009 → T010 → T011 → T012 순서로 수행한다. 실제 로그가 없으면 T011의 원인별 수정과 T012의 완료 판단을 보류한다.

## 7단계: 한/영 전환 키 전달

- [x] T013 [US1] `src/commit_editor.rs`에 일반 문자 매핑이 없는 입력기 키도 OS 번역에 전달되고 앱 단축키에 소비되지 않는 회귀 검증을 추가한다. (FR-006)
- [x] T014 [US1] `vendor/gpui/src/platform/windows/ime.rs`, `events.rs`에서 입력기 처리 키와 모드 전환 키를 일반 키 변환 전에 Windows로 전달하고 key-up 처리도 보존한다. (FR-006)
- [x] T015 [US1] 편집기 테스트와 `cargo run`의 실제 한/영 전환을 검증하고 `specs/007-fix-korean-ime/quickstart.md`에 결과를 기록한다. (SC-004)

T013 → T014 → T015 순서로 진행하며 기존 T012의 실제 입력 보존 확인도 함께 완료한다.
