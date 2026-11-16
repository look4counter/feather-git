# 데이터 모델

CommitReference.full_name은 refs/heads/...·refs/remotes/...·refs/tags/...의 정확한 참조 경로이다. 기존 name·label·is_remote는 유지한다.
RepositoryTab은 로컬·원격·태그에 대한 전체 참조 경로를 만들며 스태시에는 경로를 만들지 않는다.
reference_snapshot은 기존 Project 형식으로 대상 커밋이 포함된 기록을 반환한다. UI는 커밋 목록과 more_commits를 갱신하고 HistoryState를 다시 생성한다.
커밋 선택 과정에서 refresh_panel_data가 선택을 지우므로 대상 선택 후 현재 저장소 행 선택을 복원한다. 오류 시 기존 커밋·기록을 유지한다.
