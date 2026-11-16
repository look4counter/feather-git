# 작업 목록

## 1단계: 준비

- [x] T001 src/ui.rs의 history_row·badge_base·reference_icon_view 및 src/theme.rs의 색상 전달을 분석한다.

## 2단계: 기반

- [x] T002 specs/044-reference-badge-colors/spec.md·plan.md·contracts/ui.md의 일관성을 확인한다.

## 3단계: 사용자 이야기 1

독립 검증: 원격·로컬은 종류별 색상을 사용하고 태그·기타는 기존 행별 색상을 유지한다.

- [x] T003 [US1] src/ui.rs의 커밋 기록 배지 색상을 원격 붉은색·로컬 민트색으로 분기한다. (FR-001~003)

## 4단계: 검증 및 수렴

- [x] T004 cargo fmt --check·cargo check --offline와 색상 전달 경로를 확인하고 specs/044-reference-badge-colors/validation.md에 기록한다.
- [x] T005 specs/044-reference-badge-colors/spec.md·plan.md·tasks.md와 코드 수렴을 확인한다.

## 의존성과 전략

T001 → T002 → T003 → T004 → T005 순서로 수행한다. 수정 지점이 한 곳이므로 병렬 구현은 필요하지 않다. 표시 분기만 변경하고 독립적인 추가 기능은 도입하지 않는다.
