# 구현 계획

## 원인과 기존 흐름

ListScrollHandle.wheel은 오프셋이 변하지 않으면 이벤트를 소비하지 않아 부모 세로 스크롤이 이어받는다. 그러나 src/virtual_list.rs의 가로 전용 Div는 GPUI 기본 restrict_scroll_to_axis=false여서 세로 delta를 가로 delta로 대체한다. src/scrolling.rs도 세로 범위가 없을 때 일반 Y를 X로 바꾸는 별도 fallback이 있다.

기존 ListLoading은 150ms 지연·완료 AtomicBool·세대/타이머 취소를 제공한다. 메인 그리드에는 loading이 연결되지 않았고 참조 확장 조회는 독립 로딩 슬롯이 없다. 그래프 준비는 기존 PanelRequest와 panel_loading을 사용한다. 파일 이력 팝업은 즉시 텍스트로 로딩을 표시한다.

## 스크롤 설계

src/scrolling.rs에서 Shift일 때만 Y를 X로 변환한다. src/virtual_list.rs의 일반/고정 gutter용 두 가로 Div에 restrict_scroll_to_axis=true를 설정하여 경계에서 전달된 일반 Y를 X로 대체하지 못하게 한다. 이벤트 소비 조건은 유지해 부모 세로 전달을 막지 않는다. GPUI vendor 코드는 변경하지 않는다.

## 로딩 설계

src/ui.rs에 메인 참조 커밋 조회용 ListLoading을 추가한다. 요청 생성 시 begin, 작업 스레드 완료 시 completion 신호, 유효 결과 완료/취소/탭/프로젝트 전환 시 reset한다. 메인 그리드는 해당 로딩 또는 panel_waiting이면서 커밋 수와 준비된 HistoryState 수가 달라 실제 그래프 준비가 필요한 경우의 panel_loading만 받는다. 파일만 조회할 때는 기존 커밋을 가리지 않는다. 준비 중 비어 있는 그리드도 VirtualList를 렌더해 공통 타이머를 예약한다.

src/file_history_dialog.rs의 상태에 독립 ListLoading을 두고 생성 시 begin, 백엔드 완료 신호, finish/close reset을 연결한다. 즉시 로딩 텍스트를 제거하고 조회 중에도 빈 VirtualList를 렌더하여 같은 지연 문구를 사용한다. 기존 빈/오류/읽기 취소/세대/선택 보호는 유지한다.

## 영향과 검증

외부 의존성·Git 요청·영속 형식·아키텍처 책임을 추가하지 않는다. Constitution은 미작성 템플릿이다. 기존 수평-only 휠 fallback 테스트는 최신 축 정책으로 갱신하고 경계/부모 전달을 확인한다. 메인 참조 지연/완료/취소와 그래프 준비/파일 조회 구분, 팝업 빠른 완료/닫기 테스트를 추가한다. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 수행하고 GUI/실행 파일 잠금 한계를 명시한다.
