# 구현 계획: 패널 클릭 포커스

## 기술 맥락과 기존 구조
Rust 2024와 GPUI를 사용한다. Workspace가 참조·파일·히스토리와 뷰어 배치를 소유하고 DiffViewer가 선택·키 입력·자체 FocusHandle을 소유한다. VirtualList는 포커스를 별도로 갖지 않는다. 뷰어 키 입력은 is_focused로 차단되지만 다른 패널은 포커스를 가져가지 않는다. 커서 그리기는 창 활성 여부를 검사하지 않으며 포커스 알림 구독도 없다.

## 설계와 영향 범위
src/ui.rs의 Workspace에 세 패널의 기본 FocusHandle을 지연 생성하여 보존한다. 공통 패널 컨테이너에서 캡처 단계의 왼쪽 클릭을 처리한다. 캡처는 부모에서 자식 순서이므로 입력 자식이 최종 포커스를 얻는다. 이벤트 전파는 중단하지 않는다. 차단 상태는 기존 repository_dialog·alert·repository_busy 판정을 따른다. 네 번째 패널은 src/diff_viewer.rs의 기존 FocusHandle을 재사용한다.
DiffViewer의 기존 생성 서명을 유지하고 최초 렌더에서 포커스 획득·상실 및 창 활성 구독을 한 번 등록한다. 포커스 상실 시 드래그를 취소하고 다시 그린다. 창 비활성 중 커서를 숨긴다.

## 아키텍처 검토
Constitution은 미작성 템플릿이므로 적용 가능한 규칙이 없다. 사용자 규칙에 따라 UI 책임과 백엔드 경계를 유지하고 공개 API·데이터·의존성을 변경하지 않는다. 장식 요소의 개별 포커스나 새 단축키는 추가하지 않는다.

## 검증
cargo fmt --check, cargo test, cargo clippy --all-targets를 실행한다. GPUI 캡처 순서와 차단 검사를 코드로 확인하고 quickstart.md의 실제 창 시나리오는 실행 가능 여부를 따로 보고한다.
