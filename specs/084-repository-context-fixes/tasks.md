# 작업 목록: 저장소 컨텍스트 결함 수정

## 1단계: 준비

- [x] T001 src/commit_history_actions.rs, src/synchronization.rs, src/rebase_actions.rs, src/repository_search.rs, src/workspace_backend.rs와 vendor/gpui/src/platform/windows/platform.rs를 분석하고 plan.md에 반영한다.

## 2단계: 사용자 이야기 1 — 안전한 Git 명령

- [x] T002 [US1] src/commit_history_actions.rs에 .git2 단독·동위·상위 저장소의 Reset/Revert 및 계속·중단 회귀 테스트를 추가한다. FR-001, FR-002
- [x] T003 [US1] src/repository.rs의 공통 Git 명령 생성기를 이력 변경·src/synchronization.rs 병합 중단·src/rebase_actions.rs 리베이스 중단에 적용한다. FR-001
- [x] T004 [US1] src/synchronization.rs와 src/rebase_actions.rs에서 .git2 충돌 중단 및 비대상 메타데이터 보존을 검증한다. FR-002

## 3단계: 사용자 이야기 2 — 메타데이터 검색 제외

- [x] T005 [US2] src/repository_search.rs에 메타데이터 제외와 일반 파일·커밋 검색 보존 회귀 테스트를 추가한다. FR-003
- [x] T006 [US2] src/repository.rs·src/backend.rs·src/diff_git.rs의 메타데이터 수집 규칙을 공유하고 src/repository_search.rs에서 제외한다. FR-003

## 4단계: 사용자 이야기 3 — 터미널 컨텍스트

- [x] T007 [US3] src/workspace_backend.rs에 실제 터미널 환경 구성과 자식 Git 디렉터리 검증 테스트를 추가한다. FR-004
- [x] T008 [US3] src/workspace_backend.rs에서 Git Bash·PowerShell에 선택 저장소 환경을 전달하고 베어·Explorer 동작을 보존한다. FR-004

## 5단계: 사용자 이야기 4 — 숨김 폴더 선택

- [x] T009 [US4] vendor/gpui/src/platform/windows/platform.rs의 폴더 선택 옵션에 숨김 표시를 추가하고 Windows 빌드와 옵션 보존을 확인한다. FR-005

## 6단계: 통합 검증과 수렴

- [x] T010 전체 회귀·빌드·정적 분석을 실행하고 specs/084-repository-context-fixes/quickstart.md 및 docs/qa-2026-11-17.md를 갱신한다. FR-006
- [x] T011 specs/084-repository-context-fixes의 명세·계획·작업과 코드를 수렴 검사하고 남은 작업을 완료한다.

## 의존성과 실행 전략

T001 → US1 → US2 → US3 → US4 → 통합 검증 순서이다. 각 이야기의 재현 테스트를 먼저 실행한다. 같은 저장소 파일을 공유하므로 구현은 순차 진행한다. 독립된 테스트 그룹은 병렬 실행할 수 있다. 사용자 이야기 1이 가장 우선인 최소 단위이며 네 이야기 모두 완료한다.
