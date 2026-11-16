# 검증 안내

1. cargo test --offline, cargo fmt --check, cargo check --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행한다.
2. 새 빌드를 --backend-delay-ms 1500으로 실행한다. Open/Create/Clone 제출 직후 입력 창이 닫히고 앱 전체에 기존 애니메이션이 표시되는지 확인한다. 실제 생성/복제는 별도 테스트 저장소에서 수행한다.
3. 최근 저장소 클릭도 같은 오버레이를 사용하는지, 완료/실패 시 사라지고 실패 경고와 기존 작업 공간이 유지되는지 확인한다. 대기 중 클릭·키보드·스크롤·중복 요청을 확인한다.
4. 시작 화면의 최근 목록에서 이름·경로가 두 줄로 읽히는지 좁은/넓은 창에서 확인한다. 저장 값과 선택된 저장소가 변경되면 안 된다.
5. 이번 세션에서는 컴퓨터 사용의 Feather Git 화면 접근이 자동 승인 검토에서 거절되었으므로 GUI 검증을 수행했다고 보고하지 않는다.
