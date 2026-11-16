# 작업 목록

## 1단계: 입력 기반 (사용자 이야기 1)
- [x] T001 src/text_input.rs에 기존 편집/IME를 재사용하는 multiline 모드·줄별 표시·이동·스크롤과 관련 테스트를 추가한다.

## 2단계: 커밋·푸시 (사용자 이야기 2·3)
- [x] T002 src/committing.rs, src/repository.rs, src/backend.rs에 draft·staged-only commit/amend·선택적 push 및 부분 성공 결과를 구현한다.
- [x] T003 src/committing.rs에 격리 Git/로컬 원격 정상·오류·충돌·잠금·amend·push 거부 테스트를 추가한다.

## 3단계: 팝업 및 연결
- [x] T004 src/commit_dialog.rs, src/main.rs, src/form_controls.rs와 src/split_button.rs에 공용 스타일의 Commit 팝업·TextArea·체크박스·툴바 버튼을 구성한다.
- [x] T005 src/workspace_backend.rs와 src/ui.rs에 비동기 실행·작업 가드·진행 오버레이·결과 후 재조회·실패 초안 보존·부분 성공 알림을 연결한다.
- [x] T006 src/ui.rs 및 관련 모델에서 완료/오류/부분 성공·오래된 요청·선택/포커스 동작의 회귀를 검증한다.

## 4단계: 전체 검증 및 수렴
- [x] T007 전체 테스트·포맷·엄격한 clippy·별도 빌드를 실행하고 quickstart.md에 기록한다.
- [x] T008 명세·계획·실제 구현을 수렴 점검하고 누락을 해결한다.

## 의존성과 전략
T001 → T002 → T003 → T004 → T005 → T006 → T007 → T008 순서로 수행한다. 코드 변경은 순차 수행하고 독립 조사만 병렬 가능하다. 실제 사용자 저장소와 원격은 변경하지 않는다.
