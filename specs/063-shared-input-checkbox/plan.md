# 구현 계획

## 기술 및 기존 구조 분석
Rust/GPUI TextInput은 공통 입력 처리·IME·비밀 보호를 갖지만 저장소·설정·인증 외곽은 개별 Div이다. settings/auth 입력은 compact 모드이며 외곽 36 패딩4를 사용한다. 이를 Open Repository 기준(내부32, 패딩4, LINE 테두리1, rounded_md, 외곽 패딩 없음)으로 통일한다. src/form_controls.rs에서 input 및 labeled_input을 제공하고 모든 Entity<TextInput> 표시를 연결한다. 일반·읽기 전용·비밀번호 모두 같은 기본 높이를 사용한다. 기존 Input의 배경과 보호 상태는 그대로 사용한다.

## 체크박스 및 삭제 UI
settings_controls의 체크 표시를 form_controls로 옮겨 mark와 label 포함 checkbox를 제공한다. 환경설정 체크박스를 버튼 외곽에서 분리하고 justify_start/text_align Left를 명시한다. Diff 선택 열의 문자 기호를 공통 mark로 바꾸되 24 열 및 클릭 이벤트를 유지한다. 설정 전용 클릭 판정 유틸도 동일 모듈로 이동하고 중복 모듈은 제거한다. 원격/인증 삭제는 공통 휴지통 canvas 아이콘을 사용하고 기존 dialog::button 외형을 유지한다. 해당 행은 items_center로 통일하고 다중 원격 주소 항목은 전체 높이 기준으로 가운데 정렬한다.

## 더블클릭 데이터 흐름
TextInput 마우스 처리에서 두 번 클릭을 분기한다. ShapedLine::index_for_x로 클릭한 글자의 시작 바이트를 얻고 수평 스크롤과 비밀번호 마스크 좌표를 반영한다. 기존 text_selection::Selection::select_word의 구분 규칙을 재사용하고 문자 위치를 UTF-8 바이트 선택 범위로 변환한다. 비밀번호는 전체 선택하여 단어 경계를 노출하지 않는다. 조합 중 더블클릭은 기존 조합을 보존한다. 기본 클릭/드래그는 기존 closest_index 경로를 유지한다.

## 범위 및 불변 조건
src/form_controls.rs, text_input.rs, repository_dialog.rs, settings.rs, auth_settings.rs, diff_viewer.rs, main.rs와 불필요해진 테마/설정 컨트롤만 변경한다. 헌법은 미작성 템플릿이다. AGENTS.md의 기존 호출 흐름·계층·최소 변경을 준수하며 새 의존성과 저장 형식 변경은 없다. 읽기 전용·비밀번호·비활성 입력의 가드는 유지한다.

## 검증
기존 선택 알고리즘 테스트와 새 입력 바이트/단어 선택 테스트로 한글·영문·밑줄·숫자·공백·구두점·이모지·끝 위치·빈 문자열 및 마스크 매핑을 검증한다. 기존 IME/인증/폐기·선택 회귀 테스트와 전체 테스트, fmt, 엄격한 clippy 및 빌드를 실행한다. 사용 중인 앱을 종료하지 않고 필요시 별도 출력 파일로 빌드한다. 실제 GUI 미검증은 별도로 보고한다.
