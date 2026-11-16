# 구현 계획

## 구조와 영향

변경은 src/ui.rs의 Workspace::branches로 제한한다. 호출은 Workspace::render에서 저장소 패널로 연결되며 프로젝트 데이터는 기존 로더가 제공한다. 로컬 활성 표시는 고정되어 있고 별도 탭 상태는 없다. 로컬 목록 뒤의 원격 제목·목록 생성을 제거하고 현재 강조와 빈 상태를 유지한다. 의존성 및 공개 데이터 형식 변경 없음.

## 헌장

헌장은 미작성 템플릿이므로 규범 검사를 생략한다. 사용자 규칙에 따라 데이터 접근과 UI 책임을 유지한다.

## 검증

변경 후 branches 렌더 함수가 로컬 데이터만 참조하는지 확인하고 cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline을 수행한다. 단순 표시 삭제를 그대로 복제하는 테스트는 추가하지 않고 기존 자동 테스트와 소스 대조로 검증한다. 실제 UI 검증 여부는 결과에 명시한다.
