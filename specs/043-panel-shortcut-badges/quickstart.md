# 검증 안내

1. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
2. 작업 트리 파일 패널 클릭 시 하단에 space 배지와 선택파일 스테이지 토글이 나오는지 확인한다. 선택 파일에서 Space를 눌러 Stage/Unstage가 왕복하는지 확인한다.
3. 미스테이지 텍스트 diff 클릭 시 현재 줄/범위 선택 안내가 나오는지 확인하고 Space 체크 동작을 확인한다.
4. 상세·Staged diff·다른 패널·모달·입력 자식·창 비활성에서는 안내가 숨는지 확인한다. 로딩과 작업 중도 확인한다.
5. 기존 뷰어 내부 안내는 사라지고 Stage 버튼·구간 선택·Enter 비활성이 유지되는지 확인한다.
