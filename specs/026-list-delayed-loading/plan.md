# 구현 계획: 목록 지연 안내 공통화

## 기존 구조와 호출·데이터 흐름

Rust 2024와 로컬 GPUI를 유지한다. src/loading.rs의 DelayedLoading과 150ms 상수는 이미 공통 순수 상태다. src/ui.rs는 요청 시작 시 panel_loading을 시작하고 dispatch_pending에서 AtomicBool·타이머를 만들며 히스토리 전용 메서드로 표시를 결정한다. files에서 목록 바깥에 문구를 직접 렌더링하고 데이터가 비어 있으면 VirtualList 자체를 생성한 뒤 붙이지 않는다. finish_panel/invalidate_requests가 상태를 초기화한다.

src/virtual_list.rs의 VirtualList는 RenderOnce이며 스크롤 핸들·헤더·선택을 공통 처리한다. 다른 사용자는 참조 목록·커밋 목록·diff 목록이다. 재생성되는 RenderOnce 객체에 타이머 상태를 직접 저장하면 요청마다 타이머가 반복되므로 요청 소유자가 보존하는 공유 핸들이 필요하다.

## 설계

- src/virtual_list.rs: ListLoading 핸들을 추가한다. Rc<RefCell>로 순수 DelayedLoading, 현재 요청 번호, 요청별 Arc<AtomicBool> 완료 신호, Option<Task> 타이머를 공유한다. begin/reset이 이전 타이머를 취소하고 신호를 분리한다. 백엔드는 완료 신호만 소유하며 UI 전용 핸들을 전달하지 않는다.
- ListLoading은 150ms 설정과 기존 DelayedLoading을 재사용한다. 슬롯별 요청 번호와 완료 신호로 오래된 타이머를 차단한다. 타이머는 약한 참조만 캡처하여 소유자가 소멸하면 상태와 Task가 해제되고 순환 참조가 없다.
- VirtualList::loading(handle) 선택 빌더를 추가한다. 기본 None이므로 기존 호출부는 변하지 않는다. RenderOnce에서 pending이면 이전 행과 스크롤바를 렌더링하지 않고 헤더를 유지한다. 150ms가 지나면 공통 문구를 MUTED 색과 기존 여백으로 표시한다. 같은 요청의 Task가 있으면 추가 예약하지 않는다. 비차단 timer와 Window::spawn으로 창을 갱신하고 창 종료 오류는 정상 종료로 취급한다.
- src/ui.rs: panel_loading을 ListLoading으로 바꾸고 시작·초기화·완료 연결을 유지한다. 작업 완료 AtomicBool은 공통 핸들에서 얻는다. dispatch_pending의 전용 타이머와 files의 문구를 제거한다. 히스토리 대기 중에는 데이터가 없어도 .loading(handle)을 연결한 목록을 붙인다. 기존 고정 커밋상세·오류·빈 결과 분기는 유지한다. 기존 시간 주입 테스트 보조 메서드는 테스트 빌드에만 남긴다.
- src/loading.rs: 순수 상태·150ms 설정·기존 경계 테스트를 유지한다. 새 외부 의존성·백엔드 동작 변경은 없다.

## 헌법과 영향 범위

헌법은 미작성 템플릿이라 규범 검사를 생략한다. AGENTS.md의 계층 책임·호환성·최소 변경 원칙을 따른다. UI는 요청 수명과 결과, List는 안내 표시·타이머, 백엔드는 실제 조회를 담당한다. 기존 VirtualList 함수와 일반 목록 호출은 호환된다. 목록 외 UI나 다른 목록의 안내를 임의 추가하지 않는다.

## 검증

기존 149/150ms 상태 테스트를 유지하고 공통 핸들의 복제 공유·목록 간 독립성·빠른 실제 완료·늦은 완료·이전 요청 번호 무시·초기화 및 새 요청의 표시 초기화를 검증한다. 테스트를 먼저 작성한다. 기존 UI 지연/탐색/오류/최신 결과 테스트와 가상화·스크롤 회귀를 실행한다. 전체 테스트, fmt, check, clippy, build를 실행한다. 실행 중 앱이 exe를 잠그면 강제 종료하지 않고 링크 산출물 검증과 실행 파일 교체 제한을 명시한다. GUI 수동 검증은 수행 여부를 구분한다.

## 작업 트리 탭 연결 보완

기존 코드의 원인은 refresh_panel_data에서 file_history일 때만 begin하고 files에서도 file_history && panel_waiting일 때만 로딩 핸들과 빈 대기 목록을 붙이는 세 조건이다. 요청·완료·무효화 경로는 두 탭이 이미 공유하므로 새 상태나 타이머 없이 이 제한을 제거한다.

src/ui.rs의 유효한 패널 요청 시작 시 공통 핸들을 항상 시작한다. 변경 파일 VirtualList에는 항상 핸들을 연결하며 실제 로딩 여부는 핸들의 pending 상태가 결정한다. 데이터가 비어 있어도 panel_waiting이면 목록을 붙여 타이머가 예약되게 한다. 히스토리의 고정 상세 행과 완료·빈 결과·기존 오류 정책은 유지한다. 참조·커밋·diff 목록과 백엔드 코드는 변경하지 않는다.

기존 조회 시작을 확인하는 재현 테스트를 먼저 작성한다. 가상 시각을 사용해 작업 트리 149/150ms·빠른 실제 완료·성공/빈 결과/오류·히스토리에서 작업 트리로 전환한 뒤 이전 완료/타이머/결과 무시를 확인한다. 기존 UI 및 공통 List 회귀와 fmt/check/clippy/build를 실행한다. 실행 중 앱의 기본 exe 잠금은 강제 종료 없이 제한을 기록하고 별도 링크 산출물을 검증한다.
