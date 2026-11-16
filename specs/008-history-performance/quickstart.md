# 실행 및 검증

cargo test --offline로 회귀를 검사한다. cargo test --offline benchmark_history -- --ignored --nocapture로 준비 비용을 측정한다. cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-history.exe로 빌드한다.
실제 저장소에서 세로/가로 스크롤, 헤더 정렬, 다중 레인 원 간격, 선택·상세와 프로젝트 전환 후 스크롤 유지를 확인한다. 측정된 준비 비용은 화면 FPS와 구분한다.

## 추가 검증

cargo test --offline history_graph 로 수직선 분류, 경계, 기존 노드와 간격을 검증한다. 새 실행 파일에서 같은 저장소의 창 이동과 스크롤을 확인한다. GPU 실측이 없으면 속도 개선을 완료로 단정하지 않는다.

## old 동등화 검증

전체 테스트와 clippy, 빌드를 실행한다. cargo run으로 같은 저장소에서 이동/스크롤을 비교한다. old는 release 실행이 확인되므로 필요시 양쪽을 같은 프로필로 비교한다. 그래프 색상, 연결, 원 외경/내경, 가로 스크롤 정렬도 확인한다.
