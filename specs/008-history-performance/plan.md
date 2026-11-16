# 구현 계획: 히스토리 가상화와 그래프 간격

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

## 분석

Workspace::history는 모든 commits에 대해 Div/문자열/리스너를 매 프레임 생성한다. select_project는 rebuild_graphs를 항상 호출하여 동일 데이터의 GraphBuilder와 RowGraph까지 재구축한다. RowGraph는 경로 캐시를 가지나 이 재구축으로 캐시가 사라진다. 그래프는 45px 고정이고 graph_x가 12px 레인 간격을 더 압축하므로 13px 이상 체크아웃 원이 겹친다.
old는 uniform_list의 range에서 history_row를 생성하고 프로젝트별 그래프/스크롤 캐시를 사용한다. 현재 GPUI 공식 uniform_list 구현도 표시/측정 범위 콜백을 제공한다.

## 설계와 영향

- src/history.rs: 프로젝트별 HistoryState를 만든다. 토폴로지는 스냅샷당 한 번 계산하고 표시 행은 요청 범위에서만 준비한다. PreparedHistoryRow는 RowGraph와 SharedString 제목/작성자/날짜/해시/참조, HEAD 여부를 담는다. UniformListScrollHandle을 프로젝트별 보관한다. 현재 200개 로딩 상한 안에서 준비한 행을 재사용하며 측정 행 호출 때문에 매번 캐시를 비우지 않는다.
- src/ui.rs: 기존 graphs/graph_lanes를 프로젝트별 HistoryState 맵으로 대체한다. select/close/activate에서 재사용/해제/무효화를 구분한다. history_row로 행 구성을 분리하고 uniform_list 콜백 범위만 준비·렌더링한다. 표 헤더와 가상 목록을 공통 가로 스크롤 컨테이너 안에 두고 최소 열 너비를 유지한다.
- src/history_graph.rs: 20px 레인 간격/10px 여백/레인 수 기반 최소 열 너비를 공통 함수로 제공한다. 선/노드에 같은 좌표를 사용하고 압축을 제거한다. 기존 24px/원 크기·링·DPI 보정과 경로 캐시는 유지한다.
- src/main.rs: history 모듈 등록.

## 헌장과 제약

헌장은 미작성 템플릿으로 검사 생략. UI 표시 캐시는 UI 계층에 두며 Git 서비스와 순수 GraphBuilder 알고리즘을 변경하지 않는다. 의존성·공개 외부 API/데이터 형식·로딩/정렬 정책은 유지한다. 이전 좁은 열 압축 규칙은 사용자 겹침 수정 요구에 따라 필요한 열 너비/가로 스크롤로 대체한다.

## 검증

변경 전 200행 프로젝트 전환 준비 반복을 같은 ignored 성능 테스트로 측정한다. 원 겹침 재현 테스트를 먼저 추가한다. 이후 캐시 지연 준비/범위/재사용/교체·닫기 해제와 다중 레인 실제 노드 경계를 테스트한다. 준비 비용은 실제 사용되는 HistoryState의 범위 준비 함수로 측정하며 전체 GUI FPS 개선 수치라고 보고하지 않는다.
포맷·컴파일·전체 테스트·clippy·실행 파일 빌드를 수행하고 실제 스크롤/가로 정렬/시각 품질은 별도 미검증 여부를 기록한다.

## 추가 분석 및 설계

빈 탭은 정상이고 이력 화면만 느리다. Windows DirectX의 경로 배치는 draw_paths_to_intermediate에서 화면 크기 MSAA 중간 텍스처를 지우고 ResolveSubresource한다. 기존 수직선은 사각형이어도 PathBuilder::fill을 사용했다. old와 현재의 DirectX 렌더러는 동일하므로 공용 렌더러를 변경하지 않는다. src/history_graph.rs에서 수직선 경계를 계산하는 함수를 추출하고 paint_quad로 그린다. 캐시는 곡선 경로만 보관한다. 수직선 분류와 실제 경계 함수의 회귀 테스트를 먼저 작성하고 전체 검사한다. 외부 계약과 데이터 모델은 변경하지 않는다. 실제 프레임 지연 개선은 GPU 실측 전 확정하지 않는다.

## old 렌더링 동등화 계획

앞선 quad 혼합은 old와 다르다. old RowGraph의 선 경로 생성 블록과 paint_circle을 가져와 현재 좌표에 적용한다. 노드는 node_geometry의 기존 물리 픽셀 경계에서 중심과 반경을 구하여 외원/내원을 순서대로 Path로 그린다. src/history_graph.rs의 vertical_bounds는 새 구현에서 사용하지 않으므로 관련 테스트는 old 방식의 실제 선 경로 및 원 경로 회귀 테스트로 대체한다. 테스트 요구를 완화하지 않고 새 요구로 대체된 quad 조건만 제거한다. src/ui.rs의 가로 스크롤 안 h_full 중첩은 flex_1/min_h_0/min_w_0로 변경하고 헤더와 목록을 제한된 overflow_hidden 컨테이너 안에 둔다. 프레임워크/데이터 접근/의존성은 변경하지 않는다. 실행 프로필 변경은 이번 동등화 범위에 포함하지 않는다.

## 실제 프레임 진단 계획

임시 환경 변수로만 활성화되는 진단에서 기본 저장소 화면의 반복 프레임을 측정한다. Workspace render와 uniform_list 콜백, DirectX draw 비용을 구분한다. 외부 UI 조작 없이 앱 자체 렌더 테스트로 실행하고 자동 종료한다. 진단 후 임시 코드는 제거한다. 실제 사용자 저장소 데이터와 스크롤 입력이 포함되지 않는 경우 한계를 명시한다.

계측 결과 표시 범위 14개/200개로 가상화는 동작하며, CPU UI 준비 약 15ms 중 prepaint가 약 10ms다. GPU 제출 CPU 시간은 약 0.25ms다. Cargo.toml의 profile.dev.package.gpui와 profile.dev.package.taffy에 opt-level=3을 적용하고 동일 진단을 재실행한다. 최적화만 바꾸며 앱 소스의 개발 프로필은 유지한다.

GPUI는 debug_assertions에 따라 inspector 요소와 요소 추적 경로가 활성화된다. GPUI 패키지만 debug-assertions=false로 설정하여 old release의 비검사 렌더 경로와 맞춘다. 앱과 다른 패키지의 검사 설정은 유지한다. 동일 계측으로 재측정한다.
