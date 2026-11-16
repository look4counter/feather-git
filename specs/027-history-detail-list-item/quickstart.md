# 검증 안내

1. cargo test --offline로 상세 행 구성·선택 매핑과 전체 회귀를 확인한다.
2. cargo fmt --check, cargo check --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
3. 새 실행 파일을 --backend-delay-ms 1500으로 열고 커밋을 선택한다. 로딩 중에는 고정 상세 행이 없고, 성공 후 파일 목록 첫 행에 기존 아이콘의 커밋상세가 나타나야 한다.
4. 목록을 스크롤하여 상세 행이 함께 사라지는지, 다시 첫 행과 실제 파일을 선택했을 때 정확한 뷰어가 열리는지 확인한다. 빈 변경·오류·빠른 재선택·작업 트리 전환도 확인한다.
