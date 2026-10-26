# 기술 조사

## 전체 높이 탭 바

- 결정: 기존 GPUI의 h_full, flex_shrink_0, overflow_x_scroll과 사각 div를 사용한다. 추가 의존성은 없다.
- 근거: 기존 탭이 공용 button의 padding·rounded 스타일을 사용하여 떠 있는 형태가 됐다. 프로젝트 탭 전용 요소를 구성하면 높이를 부모에 맞출 수 있다.
- 대안: 빈 페이지를 가짜 저장소로 표현하면 불필요한 조회와 오류가 생기므로 Empty/Repository 탭 상태를 명시적으로 구분한다.

## GPUI

- 결정: 공식 GPUI 0.2.2를 고정한다.
- 근거: WindowsPlatform과 Direct3D11/DirectWrite 지원이 있다. Application, Render, div, Context listener를 사용한다.
- 대안: 커뮤니티 포크 및 Zed 전체 Git 의존성은 초기 앱에 불필요하다.
- 출처: [공식 소스](https://docs.rs/crate/gpui/0.2.2/source/).

## gix

- 결정: 로컬 소스를 확인한 gix 0.87.1을 고정한다. 참조 열거, reflog, 리비전 순회, 트리 및 status API를 사용한다.
- 근거: Git CLI 없이 읽기 전용 조회를 수행할 수 있다. 상태 결과의 write_changes는 호출하지 않는다.
- 대안: libgit2 및 Git CLI는 사용자 지정 기술에 부합하지 않는다.
- 출처: [공식 문서](https://docs.rs/gix/0.87.1/gix/).

## 비동기와 비교

- 결정: GPUI background_executor에서 저장소를 열어 소유 데이터만 반환한다. 프로젝트 ID와 요청 세대로 결과를 검증한다.
- 파일 차이는 이전/이후 내용을 similar로 비교한다. 크기·바이너리·파일 종류를 확인하고 표시를 제한한다.
- 이름 변경은 삭제/추가로 표시할 수 있다. 외부 필터 실행은 범위 밖이다.

## 영역 크기 조절

- 결정: 기존 GPUI의 canvas prepaint/paint, MouseMoveEvent.pressed_button, Window::on_mouse_event의 캡처 단계, ResizeLeftRight/ResizeUpDown을 사용한다. 추가 의존성은 없다.
- 근거: 로컬 GPUI 0.2.2 소스의 elements/div.rs, elements/canvas.rs, interactive.rs와 painting 예제에서 이벤트 API를 확인했다.
- 대안: 새 컴포넌트 의존성은 경계선 두 개에 불필요하다. 네 패널의 독립 비율은 정렬이 어긋나므로 공통 비율을 사용한다.
- 이벤트 보완: occlude는 상위 hitbox의 hover 판정도 차단한다. data_table 공식 예제와 같이 paint 단계에서 전역 이벤트를 등록하여 드래그 중 포인터가 핸들을 벗어나도 이동·해제를 처리한다.

## 초기화와 Clone 결정

설치된 Git의 전송과 인증 설정을 활용하기 위해 Git CLI를 선택했다. 셸 없이 인자를 넘기며 기존 조회는 gix로 유지한다. gix 네트워크 의존성 확장은 범위를 늘리므로 제외한다.

## zit 그래프 조사

참조 파일: D:/Project/zit/crates/zit/src/project/repository.rs의 build_commit_graph와 page.rs의 history_columns·build_commit_graph_paths·commit_graph. 입력/출력 반행 레인으로 부모를 기다리고 첫 부모는 색상을 유지하며 추가 부모는 레인과 색상을 배정한다. 행 사이 직선과 0.6 제어점 비율의 곡선을 사용한다. UI 위젯 전체를 가져오지 않고 기존 GPUI에 알고리즘과 캔버스 렌더링만 적용한다. 고정 폭 열과 가로 스크롤로 정렬을 보장한다.

## 지연 원인 조사와 개선 결정

기존 history_panel은 모든 행·셀·canvas를 매 렌더마다 생성하고 graph_width도 전체 레인을 순회했다. 커밋 클릭은 git::load로 전체 이력과 작업 상태를 다시 읽고 UI에서 모든 RowGraph를 생성했다. GPUI 0.2.2의 uniform_list 소스는 고정 높이와 제한된 높이 컨테이너에서 가시 범위만 render_items에 전달한다. 이를 적용하고 전체 순회는 백그라운드 최초 조회로 한정한다. 기존 경로 캐시는 절대 원점이 바뀌면 재생성되지만 가상화로 가시 행에만 비용이 발생하게 한다.

## 공통 스크롤 조사

GPUI uniform_list는 자체 표시 스크롤바가 없고 UniformListScrollHandle.0의 base_handle로 offset·max_offset·bounds를 얻을 수 있다. 공통 프레임으로 트랙을 만들고 비율로 offset을 설정한다. 기존 목록 중 커밋만 가상화되어 있어 나머지 목록과 상세 메시지도 같은 uniform_list로 바꾼다. 페이징은 살아 있는 iterator 작업자와 채널을 사용하여 재순회와 UI 블로킹을 방지한다.

## 불필요한 스크롤바 원인

기존 공통 프레임은 max_offset이 0이어도 바와 12px 공간을 항상 생성했다. 조건부 바는 콘텐츠 크기와 프레임 크기로 두 축을 함께 결정해야 한 축의 바가 만든 넘침이 고착되지 않는다. Diff의 기존 보수적인 문자 폭 추정도 불필요한 바를 만들 수 있어 실제 텍스트 폭을 최초 표시 때 측정하고 캐시한다.
