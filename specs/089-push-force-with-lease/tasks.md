# 작업 목록: 안전한 강제 Push

## 1단계: 준비

- [x] T001 관련 UI·백엔드·전송 호출과 Git·git2 문서를 분석하고 plan.md에 반영한다.

## 2단계: 기반 검증

- [x] T002 spec.md·plan.md·tasks.md의 안전 조건과 호환성 및 요구사항 대응을 분석한다.

## 3단계: 사용자 이야기 1

독립 검증: 일치하는 추적 값에서는 재작성 전송 성공, 불일치는 원격 보존이다.

- [x] T003 [US1] src/branch_actions.rs에 lease 성공·거부·부재 기준 재현 테스트를 먼저 작성한다. (FR-003, FR-005)
- [x] T004 [US1] src/committing.rs·src/branch_actions.rs·src/repository.rs·src/backend.rs에 기존 인증과 전송을 공유하는 lease 검증을 추가한다.
- [x] T005 [US1] src/git_action_dialog.rs·src/ui.rs·src/workspace_backend.rs에 기본 해제 체크박스와 옵션 전달·취소·대상 변경 검증을 연결한다. (FR-001, FR-002, FR-004)
- [x] T006 [US1] 관련 테스트·빌드·정적 분석을 실행하고 validation.md에 결과와 미검증 범위를 기록한다.

## 4단계: 마무리

- [x] T007 관련 소스와 명세·계획·작업을 수렴 점검한다.

의존성: T001 → T002 → T003 → T004 → T005 → T006 → T007. 단일 사용자 이야기 전체가 최소 제공 범위이며 호출 경로와 같은 파일을 공유하므로 순차 수행한다.
