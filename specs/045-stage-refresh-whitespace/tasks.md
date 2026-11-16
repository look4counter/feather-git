# 작업 목록

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

## 1단계: 준비

- [x] T001 src/ui.rs·src/workspace_backend.rs·src/diff_model.rs·src/diff_git.rs·src/diff_viewer.rs의 갱신·원문·표시 좌표를 분석한다.

## 2단계: 기반

- [x] T002 specs/045-stage-refresh-whitespace/spec.md·plan.md·contracts/ui.md의 요구사항·경계·일관성을 확인한다.

## 3단계: 사용자 이야기 1

독립 검증: Stage 완료 요청이 파일 조회만 수행하고 커밋·그래프·참조를 보존한다.

- [x] T003 [US1] src/ui.rs에서 Stage 완료의 그래프 없는 파일 갱신을 분리하고 요청·상태 보존 회귀 테스트를 추가한다. (FR-001, FR-002)

## 4단계: 사용자 이야기 2

독립 검증: 첫 행과 다른 행의 태그가 모두 보라색이다.

- [x] T004 [US2] src/ui.rs의 태그 색상을 명시적 보라색으로 고정한다. (FR-003)

## 5단계: 사용자 이야기 3

독립 검증: 공백·탭·EOF가 원문을 바꾸지 않고 정확한 위치에 표시된다.

- [x] T005 [US3] src/diff_model.rs·src/diff_git.rs에 실제 파일 끝 메타데이터와 EOF 경계 테스트를 추가한다. (FR-005, FR-007)
- [x] T006 [US3] src/diff_viewer.rs에 공백·탭·EOF 장식과 빈 파일 표시 전용 행·폭 측정을 추가한다. (FR-004~007)
- [x] T007 [US3] src/diff_viewer.rs에 장식 위치·원문 선택·복사 회귀 테스트를 추가하고 기존 Stage 테스트를 확인한다. (FR-006)

## 6단계: 검증 및 수렴

- [x] T008 전체 테스트·서식·정적 분석·빌드 결과와 실제 화면 한계를 specs/045-stage-refresh-whitespace/validation.md에 기록한다.
- [x] T009 specs/045-stage-refresh-whitespace/spec.md·plan.md·tasks.md와 코드 수렴을 확인하고 남은 작업을 완료한다.

## 의존성과 전략

T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 → T009 순서로 진행한다. 기존 파일 갱신과 원문 좌표를 재사용하고 사용자 이야기별 독립 검증 후 전체 회귀를 확인한다. T003과 T005의 파일은 독립적이지만 이번 구현은 순차 수행한다.

## 7단계: 작업 트리 F5 재조회
- [x] T010 [US4] src/ui.rs 키 입력·파일 조회·상태 보호를 분석하고 spec.md·plan.md를 갱신한다.
- [x] T011 [US4] src/ui.rs에 F5 재조회 동작과 수정키·실제 변경 반영·금지 상태 테스트를 구현한다. FR-008, FR-009.
- [x] T012 [US4] 관련 테스트·check·엄격한 Clippy·별도 빌드를 실행하고 validation.md에 기록한다. SC-003.
- [x] T013 명세·계획·작업과 코드의 수렴을 확인한다.
의존성: T010 → T011 → T012 → T013. 같은 파일의 순차 구현으로 병렬 작업 없음.
