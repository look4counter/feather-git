# 데이터 모델

Workspace.repository_operation은 None 또는 기존 Open/Create/Clone이다. begin은 중복·복원·경고 상태를 거절하고 Some으로 전환한다. 공통 완료는 None으로 바꾼 뒤 성공 프로젝트를 적용하거나 오류를 반환한다. UI는 진행 중 포커스와 복원 포커스를 따로 소유한다. 입력 대화상자는 제출 시 폐기된다.

최근 저장소의 RepositoryEntry{name,path}와 세션 형식은 유지한다. 이름·경로 텍스트는 값을 그대로 렌더링하며 레이아웃 폭과 높이만 보완한다.
