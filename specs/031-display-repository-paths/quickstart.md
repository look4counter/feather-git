# 검증 안내

1. cargo test --offline path_display로 순수 경로 변환과 내부 요청/세션 보존을 검증한다.
2. 전체 cargo test --offline, cargo fmt --check, cargo check --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
3. 최근 저장소와 현재 프로젝트 안내의 경로에서 확장 접두사가 사라지는지 확인한다. 같은 항목을 다시 열어도 기존 저장소를 재사용하고 세션 원본은 유지되어야 한다.
4. UNC는 일반 네트워크 경로로 표시하며 긴 경로·한국어·공백을 보존해야 한다. GUI 검증 수행 여부를 구분한다.
