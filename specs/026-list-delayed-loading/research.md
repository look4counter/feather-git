# 조사와 결정

- 결정: VirtualList의 선택 빌더와 지속되는 ListLoading 핸들로 공통화한다. RenderOnce 자체는 매 프레임 재생성되므로 상태를 직접 보존할 수 없다.
- 결정: 기존 DelayedLoading과 LOADING_DELAY를 재사용한다. 같은 지연 상태를 중복 구현하거나 히스토리 UI의 전용 타이머를 유지하는 대안은 제외했다.
- 결정: List가 Window::spawn 및 background_executor().timer를 예약한다. 로컬 GPUI 소스의 AsyncWindowContext::update로 닫힌 창에서도 안전하게 실패를 처리할 수 있음을 확인했다.
- 결정: Task는 핸들 상태가 소유하고 비동기 작업은 Weak만 보유한다. 타이머가 소유자를 붙잡는 순환을 피한다.
- 결정: 완료 AtomicBool을 요청별로 분리한다. 이전 백엔드가 늦게 끝나도 현재 목록을 완료 처리하지 않는다.
- 미해결 기술 항목은 없다.
