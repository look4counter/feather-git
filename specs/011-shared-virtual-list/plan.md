# 구현 계획: 공통 가상화 목록

## 요약

커밋 기록의 GPUI uniform_list 생성과 공통 크기 설정을 src/virtual_list.rs로 추출한다. 저장소·변경 파일도 같은 함수에 항목 수와 범위 렌더러를 전달한다.

## 기술 맥락

Rust 2024, GPUI 0.2.2(로컬 vendor), Windows 데스크톱. 새 의존성과 데이터 형식 변경은 없다.
현재 src/ui.rs의 branches/files는 전체 iter로 Div를 만들며, history만 cx.processor와 uniform_list를 사용한다. history의 HistoryState.prepare_rows는 범위 캐시를 사용하고 scroll 핸들을 보유한다. 데이터는 repository → Project 및 패널 조회 결과 → Workspace → 화면으로 흐른다.
공통 모듈은 데이터 계층에 의존하지 않고 GPUI와 범위만 취급한다. 행과 오류·빈 상태·제목·탭·그래프는 Workspace에 남긴다.

## 헌법 점검

현재 .specify/memory/constitution.md는 미작성 템플릿이므로 선언된 헌법 원칙은 없다. AGENTS.md의 기존 계층·호환성·최소 변경 원칙을 적용하며 충돌은 없다. 설계 후에도 의존성 방향과 공개 데이터 형식을 유지한다.

## 구조 및 영향 범위

- src/virtual_list.rs: 식별자·개수·범위 렌더러를 받는 공통 가상 목록. 범위를 현재 개수로 제한하고 빈 범위는 호출하지 않는다. 가로 크기 FitList 및 flex 크기 설정을 공유한다.
- src/main.rs: 모듈 등록.
- src/ui.rs: 세 목록을 공통 함수로 연결한다. 저장소·파일은 렌더 시점의 현재 데이터에서 범위만 순회한다. 프로젝트·탭별 고유 ID를 사용한다. 커밋의 scroll 연결, 가로 스크롤과 prepare_rows는 유지한다.
- specs/011-shared-virtual-list/: 명세·설계·검증 기록.

## 검증 방법

공통 범위 렌더링 경로의 자동 테스트로 10만 개 중 20개 생성, 빈 목록, 한 항목, 마지막 범위, 감소·역전·초과 범위를 검증한다. 기존 전체 테스트, cargo check, cargo fmt --check, cargo clippy를 실행한다. GUI 실제 스크롤과 프레임 속도는 실행 환경에서 확인 가능한 범위를 별도 보고한다.

## 단계 및 제약

명세·명확화 → 계획 → 작업 목록 → 읽기 전용 일관성 분석 → 구현 및 자동 검증 → 수렴 점검 순서로 수행한다. 가상 목록은 일정 높이의 행과 높이가 제한된 부모가 필요하며 기존 높이 36/35/커밋 ROW_HEIGHT를 유지한다. 부모를 flex와 overflow_hidden으로 제한하여 전체 콘텐츠 높이로 확장되지 않도록 한다.
## 행 선택 확장 계획

실제 구현 확인: 커밋 행만 ui.rs의 history_row에서 배경·호버·on_click을 처리한다. 저장소 현재 브랜치 강조는 Git 상태 표시이며 사용자 선택과 다르다. 파일에는 선택이 없다.
virtual_list에 제어형 선택 인덱스·기본 배경·선택 콜백을 전달한다. 공통 모듈이 반환된 행을 안정적인 절대 인덱스 ID의 컨테이너로 감싸 클릭·선택 배경·호버를 제공한다. Rc로 하나의 콜백을 가시 행 사이에 공유한다. 행 내용은 기존 크기를 유지한다.
공통 SingleSelection은 인덱스 검증·단일 선택·해제를 담당한다. Workspace는 저장소·파일 상태를 각각 소유하고 refresh_panel_data에서 해제한다. 커밋의 기존 commit 인덱스는 호환성을 위해 유지하고 공통 목록에 전달한다. 도메인 콜백은 프로젝트·탭·범위를 검증하고 기존 상세 갱신을 수행한다.
테스트는 공통 선택의 변경·반복·초과·빈 범위와 패널 갱신·커밋 선택 가드를 검증한다. 기존 가상화 테스트와 전체 테스트·타입 검사·포맷·Clippy·별도 경로 실행 파일 빌드를 수행한다. 데이터 조회·외부 API·의존성 방향은 변경하지 않는다.

## 선택 우선순위 및 입력 시점 수정 계획

현재 virtual_list의 공통 행은 선택 배경 뒤에 무조건 hover(RAISED)를 지정하여 선택색이 덮이며 on_click은 버튼 해제 이후 호출된다. vendor/gpui/src/elements/div.rs와 src/text_input.rs의 API 사용을 확인했다.
선택 행에는 호버 스타일 자체를 등록하지 않고 미선택 행에만 기존 호버를 등록한다. on_click을 on_mouse_down(MouseButton::Left)으로 교체하고 기존 콜백을 그대로 호출한다. Workspace의 범위·프로젝트 검사 및 갱신 흐름은 바꾸지 않는다. 변경 소스는 src/virtual_list.rs 한 파일이다. 새 의존성·아키텍처·데이터 형식 변경은 없다.
정적 검토로 선택 조건·이벤트 종류·버튼 및 클릭 이벤트 제거를 확인하고 기존 전체 테스트·타입 검사·포맷·Clippy와 별도 경로 빌드를 수행한다. 사소한 조건·프레임워크 이벤트 연결을 그대로 복제하는 테스트는 추가하지 않는다. 헌법은 미작성 템플릿이며 AGENTS.md 제약과 충돌하지 않는다.

## 행 높이 공통화 계획

기존 호출 관계: ui.rs의 history_row는 history_graph::ROW_HEIGHT(24)를 직접 적용하고 그래프도 같은 상수로 연결점·노드 위치를 계산한다. branches/files 행은 각각 36/35를 직접 적용한다. virtual_list는 자식의 높이 측정에 의존한다.
공통 모듈에 기본 ROW_HEIGHT=24 정의와 row_height: Pixels 인자를 추가한다. 공통 행 컨테이너에 높이 및 flex_shrink_0을 적용한다. 세 호출자는 기존 높이를 전달하고 행 내용은 h_full로 부모 높이를 따른다. history_graph는 기존 ROW_HEIGHT 이름을 공통 상수의 재노출로 유지하여 그래프 계산과 호환성을 보존한다. UI 그래프가 공통 UI 치수를 사용하는 방향으로 정리하며 데이터 계층 의존성은 바뀌지 않는다.
영향 파일은 src/virtual_list.rs, src/ui.rs, src/history_graph.rs이다. 새 의존성·도메인 기능은 없다. 기존 그래프 연결·DPI·캐시·선택·가상화 테스트, cargo check/fmt/clippy와 별도 출력 빌드를 수행한다. 단순 크기 대입을 복제하는 새 테스트는 추가하지 않는다. 헌법은 미작성 템플릿이며 기존 구조·호환성 제약 위반은 없다.

## 최신 계획: 개별 높이 제거

이 절은 앞선 행 높이 공통화 계획의 목록별 36/35 유지 및 row_height 인자 결정을 대체한다. 실제 코드에서 branches/files가 px(36.)/px(35.)를 전달하고 history가 px(ROW_HEIGHT)를 전달하는 것을 확인했다.
공통 목록 내부에서 h(px(ROW_HEIGHT))를 적용하고 row_height 인자를 제거한다. 세 호출부의 높이 인자와 ui.rs의 불필요한 ROW_HEIGHT 가져오기를 제거한다. graph는 기존 재노출을 유지하여 같은 24픽셀 정의로 계산한다. 변경 범위는 src/virtual_list.rs와 src/ui.rs이다. 데이터·선택 처리·의존성·공개 데이터 형식은 바꾸지 않는다.
회귀 검증은 기존 전체 테스트, cargo check, cargo fmt --check, cargo clippy --all-targets 및 별도 경로 실행 파일 빌드로 수행한다. 직접 GUI 확인 여부는 구분해 기록한다. 헌법은 미작성 템플릿이며 기존 아키텍처 제약과 충돌하지 않는다.
