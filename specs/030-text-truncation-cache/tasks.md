# 작업 목록: 말줄임 캐시 폭 수정

## 1단계: 준비

- [x] T001 vendor/gpui/src/elements/text.rs와 text_system/line_wrapper.rs의 실제 캐시/스타일 변경 경로를 plan.md에 분석한다.

## 2단계: 사용자 이야기 1

- [x] T002 [US1] vendor/gpui/src/elements/text_layout_cache.rs에 기존 판정을 추출하고 text.rs 운영 경로 및 src/main.rs 테스트 모듈에 연결하여 폭 확대/축소 회귀 실패를 확인한다. FR-001/005, SC-001.
- [x] T003 [US1] text.rs에 말줄임 폭 기록을 추가하고 text_layout_cache.rs에서 두 폭의 정확한 일치를 확인하도록 수정한다. FR-001~002, SC-001~002.

## 3단계: 사용자 이야기 2

- [x] T004 [US2] text.rs의 원본 스타일 구간을 불변으로 보관하고 캐시 미스 시에만 복사해 이전 말줄임의 영향을 제거한다. FR-003~004.

## 4단계: 검증과 수렴

- [x] T005 전체 테스트·앱/공급 파일 포맷·정적 분석·빌드를 실행하고 specs/030-text-truncation-cache/validation.md에 증거와 GUI 제한을 기록한다. FR-005, SC-003.
- [x] T006 명세·계획·작업과 실제 코드의 수렴을 점검한다.

## 의존성과 실행 전략

T001 → 일관성 분석 → T002 → T003 → T004 → T005 → T006 순서다. 같은 측정 경로를 순차 변경한다. 캐시 전체 제거·새 외부 의존성·최근 저장 데이터 변경은 하지 않는다.
