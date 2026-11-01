# 검증 안내

1. cargo test amend 실행으로 네이티브 수정 동작을 검증한다.
2. cargo test --bin feather-git setup::tests 실행으로 일반 커밋 회귀를 검증한다.
3. cargo check --all-targets 실행으로 화면 연결을 검증한다.
4. 앱에서 스테이징 후 커밋창의 마지막 커밋 수정을 클릭하거나 탭과 스페이스로 선택한다. 제출 후 커밋 수가 유지되어야 한다. 취소 후 다시 열면 체크가 해제되어야 한다.

## 메시지 입력창 테두리 검증

cargo check --all-targets 및 cargo fmt --check를 실행한다. 코드에서 커밋창의 메시지 편집기만 전체 둘레 테두리로 감싸는지 확인한다. 화면 검증 시 빈 상태와 여러 줄 입력 상태에서 테두리·스크롤·포커스를 확인한다.

2026-11-01 입력창 테두리 변경: 전체 대상 컴파일과 서식 검사 통과. 커밋창에서만 편집기를 감싸는 테두리를 추가했으며 공유 편집기와 포커스·입력·스크롤 코드는 유지됨을 검토했다. 실제 화면 수동 검증은 수행하지 않았다.

한 줄 입력창 스타일 통일 검증: src/settings.rs의 field와 동일한 rounded_sm, p_1, 편집기 배경색, overflow_hidden 및 flex_shrink_0 적용을 확인했다. cargo check --all-targets와 cargo fmt --check 통과. 실제 화면 수동 검증은 수행하지 않았다.

라벨 삭제 검증: 커밋창의 커밋메시지 라벨 부재와 입력창 스타일 보존을 코드로 확인했다. cargo check --all-targets 및 cargo fmt --check 통과. 실제 화면 수동 검증은 수행하지 않았다.
