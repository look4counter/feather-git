# 상태와 데이터

SyncMode는 Fetch/Merge/FastForwardOnly/Rebase이다. 요청은 프로젝트 인덱스·루트·이름·모드를 갖는다. 결과는 Git 실행 결과와 별도의 Project/PreparedHistory 재조회 결과를 갖는다. Workspace는 sync_busy만 추가하며 결과 적용 때 경로와 프로젝트 일치를 확인한다. 기존 세션 version 1에 기본값 있는 선택 필드를 추가하며 나머지 저장 형식은 유지한다.

선택 옵션은 Workspace.selected_sync_mode와 Session.pull_mode 문자열 키로 보관한다. 기본 merge, 선택 가능한 나머지는 fetch/ff-only 키이며 이전 rebase와 알 수 없는 키는 UI에서 merge로 복원한다. 기존 저장 형식 version은 유지하고 선택 필드가 없으면 기본값을 사용한다.

UI 목록은 OPTIONS의 Fetch·Merge·FastForwardOnly이며 내부 SyncMode::Rebase는 유지한다. from_option_key는 기존 rebase 키를 Merge로 복원한다. 세션 형식 변경 없음.
