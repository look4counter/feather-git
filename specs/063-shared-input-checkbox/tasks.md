# 작업 목록

## 1단계: 공통 컨트롤 (사용자 이야기 1·2)
- [x] T001 src/form_controls.rs에 입력 외곽·라벨·체크 표시·왼쪽 정렬 체크박스 및 휴지통 아이콘을 정의하고 src/main.rs의 모듈을 연결한다.
- [x] T002 src/text_input.rs의 축소 높이를 제거하고 src/repository_dialog.rs, settings.rs, auth_settings.rs, diff_viewer.rs에서 공통 입력창과 체크박스를 사용한다. 기존 보호·클릭 이벤트를 유지한다.

## 2단계: 단어 선택 (사용자 이야기 3)
- [x] T003 src/text_input.rs에서 정확한 클릭 위치와 기존 Selection 알고리즘으로 단어 선택을 구현하고 Unicode·구분자·끝·비밀번호 선택 테스트를 추가한다.

## 3단계: 삭제 표시 (추가 요구사항)
- [x] T004 src/settings.rs와 src/auth_settings.rs의 X를 휴지통으로 바꾸고 행·라벨·버튼을 세로 가운데 정렬한다.

## 4단계: 검증 및 수렴
- [x] T005 전체 테스트·fmt·엄격한 clippy·빌드를 실행하고 quickstart.md에 기록한다.
- [x] T006 명세와 구현의 수렴을 점검하고 누락을 해결한다.

## 의존성과 구현 전략
T001 → T002 → T003 → T004 → T005 → T006 순서로 수행한다. 독립 조사만 병렬 가능하며 코드 변경은 순차 수행한다. 사용자 이야기별 검증 후 전체 회귀 검사를 실행한다.
