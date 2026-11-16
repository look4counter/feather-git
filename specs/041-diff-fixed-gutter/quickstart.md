# 검증 안내

1. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
2. 긴 행이 있는 일반·과거·부분 Stage diff에서 가로 휠과 스크롤바를 조작한다. 번호·기호·체크박스·구간 Stage는 고정되고 본문만 움직여야 한다.
3. @, +, - 및 문맥 행에서 기호와 번호가 본문 기준선에 맞는지 확인한다.
4. 가로 이동 후 단어 더블클릭·드래그·복사·Home/End를 확인한다. 커서가 고정 여백 뒤에 숨지 않아야 한다.
5. 짧은 행, 빈 결과, 로딩, 좁은 패널, 상세 화면과 다른 목록의 스크롤을 확인한다.
6. 선택한 줄 Stage 버튼이 남아 있고 Enter 안내가 사라졌는지 확인한다. Enter는 Stage를 실행하지 않아야 한다.
