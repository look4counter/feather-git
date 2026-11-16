# 작업 목록

## 1단계: 앱 내부 경고 (사용자 이야기 1)
- [x] T001 src/backend.rs의 함수별 속성 전달과 테스트 전용 래퍼, src/repository.rs 및 src/graph.rs의 테스트 보조 함수를 구분한다.
- [x] T002 src/diff_model.rs, src/diff_git.rs, src/diff_viewer.rs의 미사용 필드·생성값·메서드를 제거한다.

## 2단계: 의존성 경고
- [x] T003 vendor/gpui/src/taffy.rs의 숫자 타입을 명시하고 proc-macro-error2 2.0.1을 같은 버전으로 로컬 패치한다. Cargo.toml, Cargo.lock과 패치 설명을 갱신한다.

## 3단계: 검증과 수렴
- [x] T004 src/ui.rs의 조건문 경고 정리 및 전체 테스트·정적 분석·빌드·경고 부재를 검증하고 quickstart.md에 기록한다.
- [x] T005 명세와 구현을 대조해 누락을 해결한다.

## 의존성 및 전략
T001 → T002 → T003 → T004 → T005 순서로 수행한다. 일반 빌드와 테스트 빌드를 모두 검증한다. 독립 읽기 검색만 병렬 수행 가능하며 코드 변경은 순차 수행한다.
