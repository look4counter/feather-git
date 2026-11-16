# 작업 목록

## 1단계: 준비

- [x] T001 src/diff_git.rs와 실제 Git 상태 및 로컬 gix status 구현을 분석한다.

## 2단계: 기반

- [x] T002 specs/046-clean-worktree-status/spec.md·plan.md·contracts/status.md의 일관성과 읽기 전용 제약을 확인한다.

## 3단계: 사용자 이야기 1

독립 검증: stat만 다른 깨끗한 저장소는 빈 목록이고 실제 변경은 유지된다.

- [x] T003 [US1] src/diff_git.rs에 stat 보존 회귀와 명시적 읽기 전용 외부 저장소 재현 테스트를 추가하고 구현 전 실패를 확인한다. (FR-001, FR-003, FR-004)
- [x] T004 [US1] src/diff_git.rs에서 누락된 기본 줄바꿈 설정만 메모리로 상속하고 CRLF 동등성·로컬 우선순위·실제 수정 테스트와 FastEq 크기 차이의 clean 내용 비교를 추가한다. (FR-001, FR-002, FR-005)
- [x] T005 [US1] 실제 feather-git-old와 임시 저장소 테스트를 재실행하고 실제 변경·인덱스 보존을 확인한다. (FR-002~004)

## 4단계: 검증 및 수렴

- [x] T006 전체 테스트·서식·Clippy·빌드 결과와 실제 화면 한계를 specs/046-clean-worktree-status/validation.md에 기록한다.
- [x] T007 specs/046-clean-worktree-status/spec.md·plan.md·tasks.md와 코드 수렴을 확인하고 남은 작업을 완료한다.

## 의존성과 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 순서로 진행한다. 재현 후 누락된 줄바꿈 설정 상속만 수정하고 실제 변경 회귀를 검증한다. 임시 저장소와 사용자 저장소 검증은 독립 가능하나 순차 수행한다.
