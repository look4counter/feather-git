# 구현 계획

## 기존 구조와 원인

src/diff_git.rs의 diff는 blob 원문과 작업 파일 원문을 similar.diff_lines로 바로 비교하고 렌더 시에만 CR/LF를 제거해 전체 변경처럼 보일 수 있다. src/partial_stage.rs도 원문 비교를 별도로 수행하므로 표시만 정규화하면 체크 키/쓰기 내용이 어긋난다. 텍스트 메뉴 hit는 폭이 0인 줄을 제외하므로 빈 줄에서는 열리지 않는다.

## 공통 비교와 원문 매핑

src/line_endings.rs에 CRLF만 LF로 바꾸는 비교용 Cow 텍스트와 대상 개행 선택/원문 한 줄 추가 함수를 추가한다. src/main.rs에 등록한다. diff_git와 partial_stage는 같은 comparison_text를 사용한다. 원래 before/after와 원본 검사·메타데이터는 바꾸지 않는다.

부분 Stage/폐기는 similar의 원문 tokenize_lines로 normalized diff의 old/new 인덱스를 원문 줄에 대응한다. Equal은 Stage에서 before 원문, 폐기에서 after 원문을 복사한다. 선택된 추가/복원된 삭제처럼 상대 쪽에서 가져오는 줄만 쓰기 대상의 주된 LF/CRLF로 맞춘다. 미변경 혼합 개행은 원문 그대로 보존하고 마지막 개행 유무는 유지한다. 500ms 비교/표시 제한 밖 변경 보존·원본/경로/필터 검증은 유지한다.

## 빈 줄 메뉴

src/diff_viewer.rs의 text_menu_hit에 blank 조건을 추가한다. 빈/공백만 있는 줄은 row bounds와 editor viewport 안에서 텍스트 시작점 이후를 허용하고 일반 줄은 기존 원문 폭을 유지한다. 보이는 영역/스크롤바/상태·세대 보호는 그대로 사용한다.

## 검증과 영향

기존 임시 저장소 도구로 잘못된 전체 삭제/추가를 먼저 재현하고 교차 개행 Stage/폐기·동일 내용·EOF·혼합 개행을 테스트한다. 빈 행 hit 테스트를 갱신하고 전체 회귀·서식·Clippy·빌드를 수행한다. 새 의존성/영속 형식/조회는 없다. Constitution은 미작성 템플릿이다. GUI와 실행 파일 잠금 제한은 별도 기록한다.
