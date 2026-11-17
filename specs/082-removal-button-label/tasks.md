# 작업 목록: 삭제 버튼 문구 중복 수정

## 1단계: 분석과 준비

- [x] T001 src/dialog.rs와 src/recent_repository_dialog.rs의 문구 추가 경로 및 영향 범위를 분석하고 plan.md에 기록한다.

## 2단계: 사용자 이야기 1

- [x] T002 [US1] src/recent_repository_dialog.rs에서 공통 버튼의 기본 문구를 비우고 기존 상대 배치 문구와 이벤트를 유지한다. FR-001, FR-002

## 3단계: 검증과 수렴

- [x] T003 관련 테스트·빌드·정적 분석을 실행하고 specs/082-removal-button-label/quickstart.md에 결과를 기록한다. FR-003
- [x] T004 specs/082-removal-button-label/spec.md, plan.md, tasks.md와 코드를 수렴 검사한다.

## 의존성과 실행 전략

T001 → T002 → T003 → T004 순서로 진행한다. 변경이 단일 위치이므로 병렬 구현은 필요하지 않다. 사용자 이야기 1이 전체 범위이자 최소 완료 단위이다.
