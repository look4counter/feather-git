# 구현 계획

## 기존 구조와 호출 관계
Rust 2024·GPUI 앱이다. Workspace.file_selection은 SingleSelection이며 파일 목록의 virtual_list 콜백은 행 번호만 전달한다. 같은 인덱스로 viewer_files에서 단일 ViewerTarget::File을 만든다. 쓰기 갱신의 PanelRequest.selected_file은 경로·Stage 상태 한 개를 복원한다. 파일 Space도 한 개 FileChange를 비동기로 토글한다.
DiffViewer는 파일 하나의 조회 세대·스크롤·본문 선택·체크·부분 Stage·충돌·포커스를 함께 소유한다. 여러 파일을 한 Diff로 합치면 모든 행과 쓰기 원본을 다시 매핑해야 하므로 기존 엔티티를 파일별로 재사용한다.

## 선택과 목록 설계
src/virtual_list.rs에 MultiSelection을 추가하되 기존 SingleSelection은 다른 목록에서 유지한다. 선택 집합·기준 행·현재 행을 관리하고 기존 select·selected·clear와 새 click·indices·restore를 제공한다. 선택 이벤트와 강조 집합을 받는 multi_virtual_list를 공통 구현으로 두고 기존 virtual_list는 단일 선택 어댑터로 보존한다.
src/ui.rs의 작업 트리 파일 목록에서만 수정키 클릭을 적용한다. 커밋 기록 모드는 같은 상태 객체를 단일 선택 API로 사용한다. 다중 대상은 현재 목록 순서로 ViewerTarget::Files에 전달하고 단일·빈 선택은 기존 경로를 유지한다.

## 여러 파일 뷰어 설계
src/diff_viewer.rs의 Files 대상은 파일별 기존 DiffViewer 자식을 세로로 나열하는 컨테이너다. 각 영역은 고정 높이와 자체 본문 스크롤을 갖고 바깥 목록도 세로 스크롤된다. 상단에는 선택한 파일 개수만 표시하고 파일별 경로·Stage 상태 헤더는 숨긴다. 자식 전용 헤더 숨김 상태를 두고 기존 단일 렌더의 헤더만 조건부로 생략한다. 본문과 부분 Stage 제어·오류는 유지한다. 경로·Stage 상태·저장소로 자식을 재사용하고 조회 대상이 바뀌면 기존 set_target 세대 검사로 갱신한다. 자식 ViewerChanged와 notify는 부모로 전달해 기존 Workspace 갱신과 하단 안내를 유지한다.
포커스 안내와 busy는 실제 자식 상태를 읽는다. 선택에서 빠진 자식이 쓰기 중이면 완료까지 보존해 작업 결과 알림을 잃지 않는다. 활성 자식의 상호 작용은 부모 차단 상태와 병행 쓰기에 맞춰 갱신한다. 단일 대상은 기존 렌더 경로를 유지한다.

## 배치 Stage와 선택 복원
src/workspace_backend.rs의 배치 함수는 선택 경로를 중복 제거하고 같은 경로에 미스테이지가 있으면 전체 Stage를 우선한다. 기존 diff_git::toggle_stage를 파일별로 호출하고 성공·실패·복원할 경로와 상태를 반환한다. 파일별 원자성을 유지하고 일부 실패도 명시한다.
PanelRequest.selected_files로 선택 식별자 집합을 전달하고 재정렬된 결과에서 MultiSelection을 복원한다. ViewerChanged는 기존처럼 graph 없는 파일 갱신만 요청한다. Space 완료는 성공 항목의 반대 상태와 실패 항목의 기존 상태를 복원하고 오류를 표시한다.

## 영향 범위와 아키텍처 검토
수정 범위는 src/virtual_list.rs·src/ui.rs·src/diff_viewer.rs·src/workspace_backend.rs 및 내부 테스트다. 백엔드 Git 쓰기·공개 저장 형식·상태 정렬·기존 단축키는 유지하며 외부 의존성은 추가하지 않는다. 새 대상과 요청 필드는 앱 내부 API다. Constitution은 미작성 템플릿이므로 적용 가능한 규칙은 없다.

## 검증
선택 모델의 일반·Shift·Ctrl·Ctrl+Shift·역방향·빈 집합·범위 밖 입력을 테스트한다. Workspace의 다중 대상·커밋 모드 단일 선택·stale 이벤트·식별자 복원을 검증한다. 실제 저장소의 배치 Stage/Unstage·중복 경로·부분 실패·원문 보존을 테스트한다. 기존 파일 뷰어 재사용 경계와 조회·쓰기 회귀를 확인하고 전체 테스트·서식·Clippy·빌드를 실행한다. 실제 GUI와 실행 파일 교체의 제한은 별도 기록한다.

## 중첩 스크롤 보완

여러 파일 사이의 이동을 위해 자식 목록의 스크롤이 실제로 변할 때만 이벤트를 소비한다. 경계에서는 부모의 세로 스크롤이 이어받는다. 기존 휠 오프셋·클램프·스크롤바 계산은 재사용한다.

중첩 스크롤로 화면 밖에 잘린 자식 diff는 실제 content mask와 프레임의 교집합에서만 클릭·휠을 받는다. 원래 프레임 좌표는 유지해 텍스트 행 인덱스가 변하지 않게 한다. 이 입력 경계는 별도 회귀 테스트로 확인한다.

## 후속 헤더 단순화 분석

render_files는 부모 개수 헤더, 파일별 상태 행, 자식 render의 경로 헤더를 조합한다. 상태 행을 제거하고 자식 생성/재사용 시 헤더 숨김을 설정한다. 최상위 뷰어는 기본값으로 경로 헤더를 유지한다. Git 조회·쓰기·대상 식별자는 변경하지 않는다. 서식·전체 테스트·Clippy와 빌드를 검증한다. 단순 표시 수정이므로 구현을 복제하는 테스트는 추가하지 않고 기존 대상/선택 회귀 테스트를 실행한다.

## 후속 헤더 디자인 계획

render_files의 개수 헤더는 현재 32px·px_3이며 단일 render 헤더는 36px·px_5·세로 중앙 정렬·SEMIBOLD·LINE 하단 1px 선이다. 다중 선택 헤더에 동일 속성과 제목의 flex_1/min_w_0/truncate를 적용한다. 본문·조회·쓰기·파일 개수 계산에는 영향이 없다. 기존 구조 내 스타일 수정이며 의존성을 추가하지 않는다. 서식과 Clippy로 검증하고 실제 화면 미검증 여부를 기록한다. 단순 스타일을 복제하는 테스트는 추가하지 않는다. Constitution은 기존 미작성 템플릿이며 적용할 구체 원칙이 없다.
