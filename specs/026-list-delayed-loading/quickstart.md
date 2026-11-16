# 검증 안내

1. cargo test --offline로 목록 공통 상태·로딩 경계·UI·가상화·스크롤 회귀를 검증한다.
2. cargo fmt --check, cargo check --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
3. 앱을 종료한 뒤 새 빌드로 --backend-delay-ms 1500을 사용해 커밋을 선택한다. 150ms 후 파일 히스토리 목록에 안내가 보이고 고정 커밋상세 행은 유지되어야 한다.
4. 빠른 응답·연속 재선택·작업 트리·시작 탭·프로젝트 닫기에서 이전 안내가 남지 않는지 확인한다. 다른 목록에 새 안내가 나타나면 안 된다.

추가 검증: --backend-delay-ms 1500으로 변경 파일의 작업 트리 탭을 선택한다. 150ms 후 공통 안내가 나타나고 완료 후 실제 목록 또는 빈 결과로 전환되어야 한다. 히스토리와 빠르게 교차 선택해 이전 안내나 결과가 남지 않는지 확인한다.
