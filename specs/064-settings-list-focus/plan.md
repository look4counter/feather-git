# 구현 계획

## 기존 구조 및 기술
Rust/GPUI이다. 최근 저장소 행은 p_3/gap_3/rounded_md/items_center, 제목20 줄높이·보조 문구18 줄높이와 글자11, 삭제 버튼30이다. 설정 목록은 같은 배치를 form_controls의 list_row/list_title/list_detail/list_delete로 재사용한다. 행 배경과 행 호버 배경은 추가하지 않고 버튼 호버는 기존 RAISED를 유지한다. 주소는 일반 보조 라벨로 전환하므로 설정 및 인증의 읽기 전용 주소 엔티티 캐시와 미사용 생성자를 제거한다. 삭제 범위·원격 주소 가림·인증 필터링은 그대로 유지한다.

## 체크박스
공통 체크박스의 w_full을 제거하고 align_self flex-start 및 flex_shrink_0으로 내용 너비를 유지한다. 설정 호출부의 포커스 배경을 제거한다. 박스와 문구에만 이벤트를 붙이는 기존 공통 루트가 내용 너비가 되므로 빈 행 영역은 클릭 대상이 아니다.

## 입력 포커스
TextInput render 시 최초 한 번 on_blur 및 창 활성화 구독을 등록해 보관한다. 창 비활성화에도 동일 선택 해제 경로를 사용한다. 포커스를 잃으면 cursor_offset으로 범위를 접고 selection_reversed와 is_selecting을 해제하며 notify한다. 내용·조합 상태는 변경하지 않는다. 순방향·역방향·빈 선택의 접기 규칙을 순수 선택 보조 함수로 검증한다. 구독은 엔티티 수명에 따라 정리한다.

## 탐색 버튼
repository_dialog.rs의 세 작업이 공유하는 browse 버튼에 form_controls의 folder_icon canvas를 표시한다. 텍스트만 대체하며 버튼 이벤트와 is_busy 가드를 유지한다.

## 영향과 불변 조건
form_controls.rs, settings.rs, auth_settings.rs, text_input.rs, repository_dialog.rs만 수정한다. 헌법은 미작성 템플릿이다. 새 의존성과 저장 형식 변경은 없고 UI·저장 계층 책임을 유지한다. 관련 없는 탐색 메뉴는 변경하지 않는다.

## 검증
선택 접기 및 기존 단어 선택·IME·비밀번호 테스트, 설정·원격·인증 회귀 테스트, 전체 테스트·fmt·엄격한 clippy 및 별도 실행 파일 빌드를 수행한다. 실제 GUI 검증 제한은 보고한다.
