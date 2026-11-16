# 데이터 모델

Content는 Empty·Object(ID, 모드)·Worktree이다. FileChange는 원시 경로·표시 경로·상태·staged·이전/이후 원본을 가진다. DiffLine은 이전/새 줄 번호·기호·본문을 가진다. Diff는 제목·안내·행·부분 Stage 원본·충돌 미리보기를 가진다.
ViewerTarget은 프로젝트 경로와 파일/커밋 또는 샘플 텍스트를 식별한다. 조회 세대와 작업 완료 시 검증하는 대상은 별도다. Selection은 문자 단위 anchor/cursor이며 체크 행은 조회 세대마다 초기화한다. ListScrollHandle은 뷰어의 두 축 상태를 보존한다.

충돌 표시명만 위쪽·아래쪽으로 바꾸며 Choice·Region·파일 및 인덱스 데이터 모델은 유지한다.

Stage 버튼 제거는 데이터 모델과 충돌 해결 원본을 변경하지 않는다. 뷰어에서만 사용하던 resolved 렌더링 지역 조건을 제거한다.
