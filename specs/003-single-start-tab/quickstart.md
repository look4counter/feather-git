# 실행 및 검증

1. cargo fmt --check, cargo check --offline, cargo test --offline을 실행한다.
2. cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-start-tab.exe로 빌드한다.
3. +를 반복 클릭하고 프로젝트를 선택한 뒤 다시 +를 눌러도 새 탭이 1개인지 확인한다.
4. 새 탭 화면의 작업 3개와 최근 저장소 영역을 확인하고, 버튼 클릭이 연결 예정 안내를 표시하는지 확인한다.
5. 새 탭 닫기/재열기, 모든 프로젝트 닫기와 새 탭 닫기, 앱 종료/재실행 시 새 탭 부재를 확인한다.
