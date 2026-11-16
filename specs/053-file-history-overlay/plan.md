# 구현 계획

## 기존 구조 및 호출 흐름

Workspace.FileMenuRequest는 마지막 우클릭의 프로젝트/세대/탭/행을 검증한다. 기존 파일 메뉴가 클릭 대상과 선택 집합을 분리하므로 그대로 재사용한다. 기존 커밋 행 렌더는 Workspace.history_row에 있고 HistoryColumns·HistoryState·PreparedHistoryRow·virtual_list를 사용한다. DiffViewer는 ViewerTarget::File의 historical=true에서 기존 객체 원본을 비교하며 자체 세대/취소/본문/스크롤을 보유한다.

## 조회 설계

src/file_history.rs의 읽기 전용 백엔드는 모든 로컬/원격 ref와 HEAD를 시작점으로 topological/time revwalk를 한 번 수행한다. 각 커밋과 첫 부모 tree에서 요청한 원본 경로 항목의 객체/모드만 조회하고 다를 때 FileChange와 Commit 메타데이터를 만든다. 전체 트리 diff, find_similar, rename 연결과 경로 별칭은 사용하지 않는다. 이름 변경 새 경로는 추가로 보이고 이전 경로는 현재 조회에 포함되지 않는다. 200개 제한 없이 작업 스레드에서 수행하고 커밋마다 backend checkpoint로 취소를 확인한다. 다른 브랜치의 같은 경로 변경은 유지한다. 외부 Git 명령/필터를 실행하지 않는다.

## 공통 그리드 재사용

src/ui.rs의 기존 history_row를 공유 함수 commit_grid_row로 추출하고 show_graph 옵션만 추가한다. 메인 그리드는 같은 함수로 기존 모양을 유지하고 팝업은 false로 호출한다. history_cell과 공통 열 제목도 공유한다. HistoryColumns와 HistoryState의 기존 행 준비/virtual_list·가로 스크롤을 재사용한다. 메인 그리드의 열 크기 조절 동작은 유지하고 팝업은 독립 너비/스크롤을 사용하고 같은 헤더 렌더/크기 조절 알고리즘으로 열 경계를 드래그한다.

## 오버레이 설계

src/file_history_dialog.rs의 독립 엔티티는 저장소/요청 경로·항목·HistoryState·선택·ReadSlot·자식 DiffViewer·포커스를 가진다. 생성 즉시 비동기 조회하고 완료 시 첫 행을 선택한다. 오른쪽 historical 파일 대상은 해당 커밋 전후 객체를 직접 사용하며 쓰기 기능은 비활성이다. 왼쪽은 메시지/작성자/날짜/해시를 표시하고 그래프 열은 렌더하지 않는다. Dialog를 재사용하여 좌우 영역과 닫기 버튼을 구성한다. Escape와 닫기 이벤트는 Workspace가 수신하여 읽기를 취소하고 뷰어를 deactivate한 뒤 포커스를 복원한다. Workspace는 팝업 동안 메인 상호작용을 차단하고 메인 상태/조회는 바꾸지 않는다.

## 영향과 검증

수정 범위는 백엔드 등록·새 조회/팝업·ui 그리드 공유·메뉴/모달 수명 및 관련 테스트다. 외부 의존성·영속 형식·기존 공개 API를 바꾸지 않는다. Constitution은 미작성 템플릿으로 구체 규칙 검사는 생략한다. 임시 저장소 조회 테스트와 선택/그리드 순수 상태 테스트, 전체 테스트·서식·Clippy·빌드를 실행하고 실제 GUI/실행 파일 잠금 한계를 기록한다.

PreparedHistory.without_graph는 파일 전용 결과에서 불필요한 그래프 계산을 생략하고 기존 HistoryState 행 준비를 유지한다. src/history.rs도 영향 범위에 포함한다. 공유 commit_grid_header는 기존 메인 헤더의 측정/경계 핸들을 그대로 재사용하고 팝업의 독립 열 크기 조절 콜백을 받는다.

## 후속 변경 영향 및 검증

기존 load는 전체 트리 rename 분석과 별칭 순회 후 두 번째로 커밋을 읽었다. 이를 요청 경로의 직접 tree 조회 한 번으로 줄인다. UI/공유 그리드/DiffViewer는 변경하지 않는다. 기존 rename 테스트를 현재 경로만 포함하고 과거 경로는 제외하는 재현 테스트로 변경하되 원격 브랜치의 현재 경로 변경 포함·읽기 전용·생성/삭제 원본 검증을 유지한다. 관련/전체 테스트·서식·Clippy·빌드를 확인한다. 실제 6만 커밋 저장소 시간은 별도 측정하지 않고 향상 배율을 주장하지 않는다.
