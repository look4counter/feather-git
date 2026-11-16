# 검증 안내

1. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
2. 파일 전체 Stage/Unstage와 부분 Stage 후 커밋·참조 상태와 스크롤이 유지되는지 확인한다.
3. 첫 행과 다른 행의 태그가 보라색인지 확인한다.
4. 탭·공백·한글이 있는 diff에서 장식과 가로 스크롤·더블클릭·드래그·복사를 확인한다. 원문 공백·탭만 복사되어야 한다.
5. 실제 파일 끝·중간 구간·빈 파일·마지막 개행 유무·바이너리·커밋 상세의 EOF 적용 범위를 확인한다.

## 작업 트리 F5 확인
작업 트리 파일 목록을 클릭한 뒤 외부에서 파일을 수정·추가하고 F5를 누른다. Stage·Unstaged 목록이 갱신되고 남아 있는 파일 선택과 커밋 기록이 유지되어야 한다. 빈 목록에서도 시도한다. 커밋 파일 탭·다른 패널·모달·Git 작업 중에는 실행되지 않아야 한다.
자동 검증: cargo test --offline ui::viewer_tests
