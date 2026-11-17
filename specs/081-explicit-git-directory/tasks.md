# 작업 목록: Git 디렉터리 직접 지정

**입력**: [명세](spec.md), [계획](plan.md), [조사](research.md), [모델](data-model.md), [계약](contracts/dialog.md)

## 1단계: 준비

- [x] T001 기존 호출 흐름·Git 라이브러리 제약·검증 범위를 조사하고 specs/081-explicit-git-directory/plan.md에 반영한다.

## 2단계: 공통 기반

- [x] T002 src/backend.rs의 기존 I/O 등록을 유지하며 src/repository.rs의 Project에 작업 폴더를 분리하고 snapshot 식별 경로를 Git 디렉터리로 통일한다.

## 3단계: 사용자 이야기 1 — 생성

**독립 검증**: 기본·사용자 지정 경로, 기존 대상 보존, 부모 이름과 선택 기본값을 검증한다.

- [x] T003 [US1] src/repository.rs 및 src/repository_dialog.rs에 경로·이름·생성 회귀 테스트를 추가한다.
- [x] T004 [US1] src/repository_dialog.rs, src/ui.rs에서 폴더 선택 기본값과 작업별 이름 계산을 적용한다. FR-001, FR-002
- [x] T005 [US1] src/repository.rs에서 최종 Git 디렉터리 생성과 부모 연결 및 대상 보호를 구현한다. FR-003

## 4단계: 사용자 이야기 2 — 복제

**독립 검증**: 로컬·빈 소스·다중 브랜치 복제와 기존 파일 보존을 검사한다.

- [x] T006 [US2] src/repository.rs에 기본·사용자 지정·빈 소스·다중 브랜치·충돌 복제 테스트를 추가한다.
- [x] T007 [US2] src/repository.rs에서 직접 Git 디렉터리 복제와 원격 추적 및 부모 체크아웃을 구현한다. FR-004

## 5단계: 사용자 이야기 3 — 열기 및 최근 저장소

**독립 검증**: 같은 부모의 두 저장소 식별·최근 목록·활성 탭 복원과 작업 파일 기능을 검사한다.

- [x] T008 [US3] src/session.rs, src/workspace_backend.rs, src/repository.rs에 경로 변환·중복 제거·다중 저장소 복원·삭제 보호 테스트를 추가한다.
- [x] T009 [US3] src/session.rs, src/workspace_backend.rs에서 이전 세션 변환과 Git 디렉터리 식별을 적용한다. FR-005, FR-006
- [x] T010 [US3] src/ui.rs, src/workspace_backend.rs, src/file_history.rs, src/file_history_dialog.rs의 작업 파일 경로를 workdir 기준으로 처리한다. FR-007
- [x] T011 [US3] src/repository.rs, src/diff_git.rs, src/partial_stage.rs에서 메타데이터 보호와 공유 작업 폴더 삭제 거절을 검증·구현한다. FR-007

## 6단계: 통합 검증

- [x] T012 관련 테스트·전체 테스트·빌드·정적 분석을 실행하고 specs/081-explicit-git-directory/quickstart.md에 결과와 제한을 기록한다. FR-008
- [x] T013 specs/081-explicit-git-directory/spec.md, plan.md, tasks.md와 구현을 수렴 검사하고 남은 작업을 완료한다.

## 의존성과 실행 전략

T001 → T002 → 사용자 이야기 1 → 사용자 이야기 2 → 사용자 이야기 3 → 통합 검증 순서이다. 각 이야기에서 테스트를 먼저 추가하고 구현 후 확인한다. 최소 동작 단위는 생성이며 세 이야기 모두 최종 완료 범위이다. 동일 파일 수정이 많으므로 순차 진행한다. 독립된 세션 테스트와 대화상자 테스트는 별도 파일에서 병렬 실행 가능하다.

## 7단계: 신규 Git 디렉터리 숨김

- [x] T014 [US1] src/repository.rs에 Create·Clone 숨김, 속성 보존·멱등성·오류, Open 비변경 테스트를 추가한다. FR-009
- [x] T015 [US1] src/repository.rs에서 Windows 숨김 속성을 보존적으로 적용하고 부분 성공 오류를 전달한다. FR-009
- [x] T016 관련 테스트·빌드·정적 분석 후 specs/081-explicit-git-directory/quickstart.md에 결과를 기록하고 수렴 검사한다. FR-008, FR-009

의존성: T014 → T015 → T016. 기존 완료 작업은 유지한다.
