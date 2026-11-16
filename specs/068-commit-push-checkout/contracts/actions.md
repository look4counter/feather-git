# 동작 계약

Commit 확인은 항상 활성 스타일이며 상태 준비/실행 오류는 inline으로 알린다. 빈 입력을 백엔드에서 거부하되 초안은 보존한다. checkout_reference는 local이면 symbolic HEAD, remote/tag면 해당 commit id의 detached HEAD를 저장한다. 안전 checkout 충돌은 참조와 파일을 덮어쓰지 않는다. 성공/실패 후 상태 재조회는 기존 ToolbarRequest 계약을 따른다. Push는 no-op과 실제 오류를 구분하고 신규 원격 참조가 필요하면 기존 전송 경로를 유지한다.
