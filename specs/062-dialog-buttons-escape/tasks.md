# 작업 목록

## 1단계: 공통 기반 (사용자 이야기 1)
- [x] T001 src/dialog.rs에 Open Repository 기준의 공통 버튼 스타일과 주/보조 역할을 정의한다.

## 2단계: 전체 팝업 통일 (사용자 이야기 1)
- [x] T002 src/repository_dialog.rs, encoding_dialog.rs, alert.rs, file_history_dialog.rs를 공통 버튼으로 전환한다.
- [x] T003 src/settings.rs, auth_settings.rs, ui.rs, recent_repository_dialog.rs, diff_viewer.rs를 공통 버튼으로 전환하고 삭제 누름 동작 및 비활성 가드를 유지한다.

## 3단계: ESC (사용자 이야기 2)
- [x] T004 src/ui.rs의 환경설정 ESC를 기존 중첩 닫기 경로에 연결하고 모든 대상 팝업의 취소·조합·작업 중 가드를 점검한다.

## 4단계: 검증과 수렴
- [x] T005 전체 테스트·포맷·정적 분석·빌드를 실행하고 quickstart.md에 기록한다.
- [x] T006 명세와 구현을 수렴 점검하고 누락을 해결한다.

## 의존성과 구현 전략
T001 → T002 → T003 → T004 → T005 → T006 순서로 진행한다. 일반 팝업과 중첩 팝업을 각각 확인한다. 독립 검색만 병렬 수행 가능하며 구현은 순차 수행한다.

## 5단계: 폐기 팝업 확인·취소 순서

- [x] T007 src/ui.rs의 공용 폐기 팝업 footer를 [확인][취소]로 바꾸고 기존 이벤트를 유지한다.
- [x] T008 폐기 회귀 테스트·포맷·정적 분석·빌드 및 명세 수렴을 검증한다.

T007 다음 T008을 수행한다.
