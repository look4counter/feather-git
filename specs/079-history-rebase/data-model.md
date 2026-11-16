# 데이터 모델

- RebaseRequest: target(선택 커밋), expected_head(현재 HEAD), expected_reference(현재 로컬 브랜치), reverse_branch(역방향일 때 로컬 참조).
- RebaseFailure: 사용자 문구와 실제 영향 경로. 원래 오류는 체인으로 보존한다.
- ToolbarAction: RebaseCommit, ContinueRebase, AbortRebase. 계속·취소는 예상 HEAD를 가진다.
- MenuItem.children: 자식 메뉴 목록. 자식이 없는 기존 메뉴는 같은 동작이다. 열린 자식 패널은 위치·선택을 가지며 키보드 깊이로 탐색한다.
- 기존 취소 확인 모델에 작업 종류를 더하여 동일 확인·취소와 포커스 수명을 공유한다. 기본 종류는 병합이다.
- 상태 전이: 정상 → 실행 → 정상 또는 리베이스 충돌 대기 → 해결/Stage → 계속 → 정상 또는 추가 충돌. 취소는 재배치 대상 브랜치의 시작 상태로 복귀한다.

역방향 복귀 문맥은 시작 브랜치·시작 HEAD·재배치 브랜치·재배치 전 tip이다. 앱 세션이 아닌 해당 rebase-merge 디렉터리에 작업 동안만 JSON으로 보관하고 실제 rebase orig_head와 검증한다.
