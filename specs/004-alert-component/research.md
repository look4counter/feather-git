# 조사 결과

GPUI의 absolute 요소와 occlude는 팝업 오버레이를 지원한다. track_focus와 on_key_down으로 모달 키 입력을 처리하며 Window::focused는 이전 포커스 복원에 사용할 수 있다. 기존 앱에는 모달/공통 테마 모듈이 없으므로 Alert의 업무 독립성과 색상 중복 방지를 위해 기존 색상만 theme.rs로 추출한다. 네이티브 시스템 alert는 앱 테마와 재사용 UI 컴포넌트 요구에 맞지 않아 사용하지 않는다.
