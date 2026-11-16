# 데이터 모델

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

RepositoryWorkState는 operation(선택적 Git 작업 종류), has_conflicts(미해결 인덱스 충돌), error(선택적 조회 오류)를 가진다. 기본값은 작업·충돌·오류 없음이다.
Project.work_state는 저장소별 최신 상태다. PanelResult는 상태 조회 결과를 포함한다.
Workspace.sync_operation은 실행 중 작업 이름이며 sync_busy와 함께 사용하고 유효 완료 시 초기화한다. 커밋은 기존 commit_busy로 표시한다.
실행 → 충돌 해결 필요 → 완료 대기 → 정상으로 전이한다. 오류는 정상과 구별한다. 영속 형식 변경 없음.

병합 취소 확인 상태는 프로젝트·경로·예상 HEAD를 가진다. ToolbarAction::AbortMerge는 예상 HEAD를 전달한다. 확인은 한 번만 소비하고 백엔드에서 실제 병합 상태와 HEAD를 재검사한다.

표시 개선은 데이터 모델·상태 전이·확인 소비를 변경하지 않는다.
