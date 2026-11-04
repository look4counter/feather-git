# 작업 목록: Windows 크레이트 업데이트

## 1단계: 준비

- [x] T001 Cargo.toml, Cargo.lock 및 기존 사용자 변경을 확인한다.

## 2단계: 기반 점검

- [x] T002 specs/011-update-windows-crate/spec.md, plan.md와 본 작업 목록의 일관성을 분석한다.

## 3단계: 사용자 스토리 1 (P1)

**목표**: 지정 버전으로 기존 Windows 기능을 컴파일한다.
**독립 검증**: 직접 의존성 0.62.2 및 전체 대상 컴파일 성공.

- [x] T003 [US1] Cargo.toml의 windows 버전을 0.62.2로 변경하고 기능 플래그를 보존한다. (FR-001, FR-003)
- [x] T004 [US1] Cargo.lock을 갱신하여 직접 의존성 0.62.2를 반영한다. (FR-002)
- [x] T005 [US1] cargo check --locked --all-targets로 src/의 Windows 호출 호환성을 확인하고 필요한 최소 수정만 적용한다. (FR-003, SC-001, SC-002)

## 4단계: 마무리

- [x] T006 specs/011-update-windows-crate/quickstart.md에 검증 결과를 기록하고 명세·계획·작업과 구현을 수렴 점검한다.

## 의존성 및 구현 전략

T001 → T002 → T003 → T004 → T005 → T006 순서로 수행한다. 사용자 스토리 1이 최소 완성 범위이며 공유 의존성 파일을 수정하므로 병렬 구현 작업은 없다. 최종 문서 검토와 변경 범위 확인은 독립적으로 수행할 수 있다.
