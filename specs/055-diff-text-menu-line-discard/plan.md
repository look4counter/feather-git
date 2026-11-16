# 구현 계획

## 기존 구조와 분석

DiffViewer는 패널 두 렌더 루트의 Right 핸들러로 체크 메뉴를 연다. selectable_text의 canvas는 실제 shaped 원문 폭/원점과 content mask를 알고 있으며 전역 캡처로 선택을 처리한다. 이 hit 경계에 Right 처리를 옮긴다. 기존 key_down의 copy와 Selection.text는 원문을 복사한다.

부분 Stage는 Preview.selected_content로 전체 원문 diff를 순회하며 체크 키를 적용하고 apply가 경로·인덱스·필터·원본을 검증한다. 부분 폐기는 같은 원문 diff와 체크 키를 쓰되 삭제 체크는 복원, 추가 미체크는 보존하도록 공통 선택 알고리즘을 확장한다. 표시 5000줄 제한 밖 변경도 보존한다.

## 메뉴 설계

src/diff_viewer.rs에서 실제 line bounds/editor viewport/text origin/width로 hit를 확인하고 Right일 때 공통 복사 메뉴를 연다. 기존 체크 요청이 가능한 경우만 추가한다. 선택은 움직이지 않고 generation/대상 검증 뒤 동일 복사 함수로 Selection.text를 쓴다. 패널 루트 Right 핸들러는 제거한다.

## 부분 폐기 백엔드

src/partial_stage.rs의 기존 Preview 알고리즘과 경로/필터/원본 검증을 공유한다. discard 선택은 체크 키만 반대로 적용하고 원래 after를 기준으로 나머지를 보존한다. src/diff_git.rs에 기존 IndexLock/안전 경로/원본 비교를 재사용하는 작업 파일 쓰기를 추가한다. 인덱스 잠금 아래 기대 인덱스/작업 파일을 재검증하고 같은 폴더 임시 파일에 기록·동기화 후 교체한다. 권한을 보존하고 신규 파일 전체 폐기는 remove_file만 사용한다. 인덱스에는 쓰지 않는다. 변환 필터/바이너리·링크는 거부하고 실패는 숨기지 않는다. src/backend.rs의 래퍼에 등록한다.

## 확인 UI

DiffViewer가 조회 세대·파일·원문·체크 줄과 WeakEntity를 가진 확인 요청 이벤트를 보낸다. 다중 뷰어 부모가 기존 ViewerChanged 패턴처럼 전달하고 Workspace가 기존 폐기 Dialog를 재사용한다. 확인 중 메인 상호작용을 차단하고 취소/Escape/Enter는 쓰지 않는다. 명시적 폐기 버튼 이후 원래 뷰어의 세대/대상을 검증한 권한 있는 요청만 기존 mutate를 실행한다. 성공/실패·파일 갱신은 기존 처리와 같다. 새로운 의존성/영속 형식은 없다. Constitution은 미작성 템플릿이다.

## 검증

순수 hit/원문/선택 알고리즘과 실제 임시 저장소의 Stage/작업 파일/다른 파일 보존·개별 줄/신규/삭제/CRLF/EOF·표시 밖 변경·원본/인덱스/잠금 실패를 검증한다. 확인과 다중 전달의 수명/세대 경로를 검토하고 전체 테스트·서식·Clippy·빌드를 실행한다. 실제 GUI/클립보드 미검증은 기록한다.
