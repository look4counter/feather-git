# 데이터 모델

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

- CommitHistoryRequest: 대상 커밋 ID·예상 HEAD ID·선택적 현재 참조. detached HEAD는 참조 없음으로 구분한다.
- ResetMode: Soft·Mixed·Hard. 기본값 Mixed.
- 작업 확인 문맥: 프로젝트 ID·경로·HEAD·브랜치·선택적 기록 행 ID와 커밋·보류 작업. 확인/취소로 한 번 소비한다.
- 팝업 옵션: 단순 확인·Revert no-commit·Reset 단일 선택. 초기값은 팝업마다 새로 생성한다.
- 실패: 원인 문자열과 충돌 경로. 성공/실패 모두 기존 ToolbarResult 갱신 계약을 따른다.

- 로컬 새로고침 작업: 선택 파일의 원시 경로·Stage 구분을 보관하고 기존 ToolbarResult로 전체 로컬 스냅샷을 교체한다. 현재 프로젝트만 대상으로 하며 원격 작업은 없다.
- SearchPanel: 소유 저장소 경로·입력·검색 세대·ReadSlot·ListLoading·결과·선택·오류·활성 상태. 새 프로젝트는 새 엔티티를 사용한다.
- 검색 결과: 커밋 ID/일치 메시지 또는 파일 경로/내용 일치 여부. 파일은 경로로 중복을 합친다.
- 검색 이벤트: 결과의 세대와 커밋 ID 또는 파일 경로. Workspace는 엔티티·세대·경로·프로젝트를 검증한다.

## 후속 구현을 반영한 모델

- RevertContext: CommitHistoryRequest와 no_commit을 보관하는 앱 내부 JSON 문맥. 저장소 메타데이터의 feather-git-revert.json에 기록하고 완료·취소 시 제거한다. 재시작 후에도 no-commit 의도를 보존한다.
- AbortOperation: Merge·Rebase·Revert를 구분하는 취소 확인 문맥. 예상 HEAD와 실제 작업 상태를 확인한 뒤 한 번 실행한다.
- ToolbarAction: ContinueRevert·AbortRevert가 예상 HEAD를 전달하고 기존 ToolbarResult 갱신 경로를 재사용한다.
- 검색 선택: 세대뿐 아니라 현재 선택 커밋 ID와 엔티티·프로젝트·경로를 검사한다. 다른 결과 선택 후 도착한 이전 커밋 이동은 적용하지 않는다.
- 그래프 추가 부모: 입력의 waiting_commit_id가 부모와 같은 후보 중 최소 to_lane 및 그 색을 재사용한다. 후보가 없을 때만 레인·색을 추가한다. 첫 부모는 현재 marker 레인을 유지한다.
