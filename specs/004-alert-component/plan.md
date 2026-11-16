# 구현 계획: 재사용 alert 팝업

## 분석과 기술적 맥락

Rust 2024/GPUI 0.2.2. Workspace::start_page()의 세 버튼은 안내만 바꾸며 별도 모달이 없다. 루트 render()는 전체 창의 flex 열이다. 팝업을 루트의 마지막 absolute 자식으로 배치하면 기존 화면 레이아웃을 바꾸지 않고 덮을 수 있다. 테마 상수는 ui.rs에 있어 재사용 컴포넌트가 ui에 역의존하지 않도록 분리한다.

## 설계와 영향 범위

- src/theme.rs: ui.rs의 기존 색상 상수만 옮기고 ui/alert가 공유한다. 색 값과 다른 화면 스타일은 바꾸지 않는다.
- src/alert.rs: Alert가 제목·메시지·확인 문구를 보관하고 render(focus, on_dismiss)로 GPUI 요소를 만든다. 닫기 콜백은 확인 클릭과 Enter/Esc에서 공유한다. Workspace 및 저장소 의존성은 없다.
- 모달은 absolute 전체 영역의 어두운 배경, 중앙 400px/가용 너비 제한 카드, 줄바꿈 메시지와 스크롤 가능한 본문, 확인 버튼으로 구성한다. occlude와 이벤트 전파 차단으로 배경 입력을 막고 외부 클릭으로는 닫지 않는다.
- src/ui.rs: Option<Alert>, 팝업 FocusHandle과 이전 FocusHandle을 메모리 상태로 보관한다. Create는 테스트 alert를 열고 포커스를 이동한다. 루트는 relative로 바꾸고 마지막 자식에 alert를 렌더링한다. 약한 Workspace 참조의 콜백으로 닫기·포커스 복원·notify를 수행한다. Open/Clone은 기존 안내를 유지한다.
- src/main.rs: alert/theme 모듈 등록.

## 헌장 및 아키텍처

헌장은 미작성 템플릿이므로 검사 생략. 의존성 방향은 ui → alert/theme, alert → theme/GPUI이며 업무 로직과 저장소 데이터 접근은 추가하지 않는다. 기존 main → ui → sample/graph 구조, 공개 API 및 데이터 형식은 유지한다. 외부 의존성은 추가하지 않는다.

## 검증

Create 진입/중복 열기/닫기/재열기/새 인스턴스 상태와 사용자 지정 메시지 보존을 자동화로 확인한다. 키보드 닫기 키 판별도 테스트한다. 포맷·컴파일·전체 테스트 및 별도 이름 빌드를 수행한다. 실제 배경 차단, 줄바꿈과 포커스 동작은 화면 도구 제한에 따라 미검증 여부를 명시한다.
