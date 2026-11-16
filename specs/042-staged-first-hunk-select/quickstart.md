# 검증 안내

1. cargo test --offline, cargo fmt --check, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
2. 실제 창에서 앞선 이름의 미스테이지 파일과 뒤쪽 이름의 Staged 파일을 준비하고 Staged가 먼저 나오는지 확인한다. 각 행의 diff가 맞아야 한다.
3. 여러 구간이 있는 미스테이지 diff에서 구간 선택을 누르면 해당 구간의 변경 줄만 체크되어야 한다. 재클릭하면 해제되고 다른 구간은 유지되어야 한다.
4. 구간 선택만으로 파일 상태가 바뀌지 않고 선택한 줄 Stage를 누를 때만 반영되는지 확인한다. 커밋 기록 목록과 Enter 비활성 정책도 유지되어야 한다.
