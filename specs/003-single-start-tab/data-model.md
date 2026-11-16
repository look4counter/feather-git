# 데이터 모델

- start_tab_open: 새 탭 존재 여부. 초기 false, 열기 true, 닫기 false.
- start_tab_active: 새 탭 선택 여부. true이면 start_tab_open도 true여야 한다.
- project/open_projects: 기존 프로젝트 선택과 순서. 새 탭 활성 중에도 이전 프로젝트 선택을 유지한다.
- 수명: Workspace 인스턴스에만 속한다. 파일 쓰기나 직렬화를 추가하지 않는다.
