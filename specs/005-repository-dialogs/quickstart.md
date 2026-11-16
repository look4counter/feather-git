# 실행 및 검증

1. cargo fmt --check, cargo check --offline, cargo test --offline을 실행한다.
2. cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-repositories.exe로 빌드한다.
3. + → Create에서 없는 경로를 입력하고 이름 자동 변경/수동 수정 후 확인한다. 빈 Git 프로젝트가 열려야 한다.
4. 새 탭 Open에서 기존 저장소를 선택하고 확인한다. 성공 Alert 없이 실제 이력·상태의 프로젝트로 전환되는지 확인한다. 잘못된 폴더에서는 입력을 유지한다.
5. Clone에서 로컬 테스트 저장소를 소스로 지정하고 빈 대상을 선택해 복제한다. 무효 소스 및 비어 있지 않은 대상은 Alert여야 한다.
6. 탐색 취소, Tab/Shift+Tab, 한글 IME/붙여넣기, Enter/Esc, 오류 Alert 복귀와 작업 중 중복 제출을 확인한다.
