# 검증 안내

1. cargo test --lib repository_watch로 시간·병합·무시 필터 및 Windows 실제 감시를 검증한다.
2. cargo check --all-targets, cargo fmt --check 및 cargo clippy --all-targets를 실행한다.
3. 앱과 외부 편집기에서 같은 저장소를 연다. 파일을 저장하면 Stage와 선택 Diff가 자동 반영되어야 한다.
4. 외부 커밋·브랜치 전환 후 이력과 참조가 갱신되어야 한다.
5. 무시 폴더에서 대량 파일을 만들 때 상태 조회가 반복되지 않아야 한다.
6. 비활성 저장소를 수정한 후 탭을 열면 누적 변경을 한 번 반영해야 한다.
7. 커밋 메시지를 입력하는 중에는 갱신을 지연하고 닫은 후 반영해야 한다.
8. 탭을 닫으면 감시 스레드와 핸들을 해제해야 한다.

## 실제 검증 결과

2026-11-01 기준:

- 자동 감지 테스트 5개 통과: 500ms·2초 시간 제어, 1,000개 이벤트 병합, 비활성·진행 중 지연, 실제 파일 생성·이름 변경·삭제, 무시 빌드 파일 1,000개와 생성된 내부 .gitignore 필터링, 무시 규칙 변경, 무시 규칙에 걸린 추적 파일, 인덱스·참조 및 연결된 worktree 감시와 종료.
- 라이브러리 테스트 58개 통과, 별도 하위 프로세스용 테스트 1개 제외.
- 앱 테스트 158개 통과, 1개 제외, 기존 단축키 테스트 1개 실패. 실패는 src/shortcut.rs의 dialog_controls_share_bindings_and_contextual_confirmation이며 모든 -tab 접미사 명령을 금지하는 검사에 기존 new-tab 명령이 걸린다. 이 기능에서 해당 파일은 변경하지 않았다.
- cargo check --all-targets, cargo clippy --all-targets -- -D warnings 및 cargo fmt --check 통과.
- 시스템 임시 폴더 접근 제한을 피하려면 .build/test-temp를 생성하고 TEMP와 TMP를 해당 절대 경로로 지정한 뒤 테스트한다.
- 실제 앱에서의 수동 GUI 검증은 수행하지 않았다. 갱신 범위·선택 및 스크롤 핸들 보존·세대 검증·모달 및 쓰기 작업 차단은 코드로 검토했다.
