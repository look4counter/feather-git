# 구현 계획

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

## 기존 구조와 호출 관계
Rust 2024·GPUI 앱이다. Workspace의 파일 Space 쓰기와 DiffViewer 쓰기는 ViewerChanged → refresh_file_data → PanelRequest로 연결된다. 실제 execute는 작업 트리에서 load_stage만 호출하지만 refresh_file_data는 history 캐시가 없거나 길이가 다르면 graph도 요청한다. Stage 완료 경로에서는 이 그래프 준비까지 금지해야 한다. 참조·저장소 전체 조회는 원래 별도 경로이므로 유지하고 요청 회귀로 고정한다.
커밋 배지는 ReferenceKind로 원격·로컬 색을 구분하지만 태그는 첫 행 강조색 분기로 빠진다. 명시적 태그 보라색 분기를 추가한다.
DiffViewer는 raw의 탭을 네 공백으로 확장해 글자와 선택·커서를 그린다. 기존 displayed_byte·original_column·original_character_column이 원문 문자 좌표와 화면 바이트 좌표를 변환하므로 이 텍스트를 기호 문자열로 바꾸지 않는다.

## 설계와 영향 범위
src/ui.rs에 refresh_file_data 요청 생성의 그래프 준비 여부를 분리한다. 일반 탐색은 기존 정책을 유지하고 viewer_changed의 Stage 완료만 graph 없는 파일 갱신을 요청한다. 기존 파일 선택 보존 경로를 재사용한다. 같은 파일에서 태그 분기만 추가한다.
src/diff_model.rs의 Diff에 eof_rows와 empty_eof 표시 메타데이터를 추가한다. src/diff_git.rs는 UTF-8 이전·이후 파일의 총 줄 수를 계산하고 실제 마지막 줄이 생성된 행에만 EOF 메타데이터를 기록한다. 제한으로 중단된 결과에는 생성되지 않은 EOF를 추가하지 않는다.
src/diff_viewer.rs는 파일 diff에서만 공백·탭 장식을 별도 MUTED 텍스트로 겹쳐 그린다. 원문 셰이핑과 4칸 탭을 유지하여 선택·복사·Stage 데이터는 바꾸지 않는다. EOF도 마지막 원문 글자 뒤에 장식으로 그리며 폭 측정에 포함한다. 빈 파일은 목록 표시 행만 추가하고 원문·선택·Stage 행에는 추가하지 않는다. 커밋 상세는 기존 표시를 유지한다.

## 아키텍처 검토
새 외부 의존성이나 공개 API 변경은 없다. Diff 메타데이터는 내부 모델의 추가 필드이며 모든 기존 원문 데이터는 보존한다. UI 표시와 Git 데이터 처리를 기존 책임 안에 둔다. Constitution은 미작성 템플릿이므로 적용 가능한 규칙은 없다.

## 검증
Stage 완료 시 graph 없는 작업 트리 요청과 참조·커밋 상태 보존을 테스트한다. EOF는 마지막 개행 유무·추가·삭제·빈 텍스트·중간 구간·바이너리에서 검증한다. 공백·탭 표시 위치와 유니코드 선택·복사·기존 부분 Stage 회귀를 검증한다. cargo fmt --check, cargo test --offline, cargo clippy --offline --all-targets, cargo build --offline를 실행하고 실제 GUI 및 실행 파일 교체의 환경 제한을 기록한다.

## F5 작업 트리 재조회 계획
분석: focusable_panel(1)이 files_key_down에 키 입력을 전달하며 활성 창·파일 패널 포커스·DiffViewer 작업 중 여부를 검사한다. viewer_changed는 기존 파일 선택을 보관한 후 request_file_data(false)를 요청한다. PanelRequest는 백그라운드 조회와 세대 검증·work_state·오류 반영을 제공한다.
설계: 수정키 없는 F5 판별 함수를 추가하고 files_key_down에서 별도 분기한다. request_worktree_refresh는 화면·작업 트리·저장소 경로·작업/모달 상태를 검증하고 viewer_changed를 재사용한다. 성공적으로 요청한 경우 전파를 중지하고 notify한다. UI 렌더링 I/O·백엔드 변경·추가 의존성 없음. 변경 범위는 src/ui.rs 및 내부 테스트다.
검증: 임시 실제 저장소에 외부 파일 추가 후 재조회 결과와 선택·커밋/참조 세대 유지, 금지 상태·수정키를 자동 검증한다. check·엄격한 Clippy·별도 빌드 실행. 실제 F5 입력 GUI는 앱 접근 제한으로 별도 미검증이다. 헌법은 기존 미작성 템플릿이며 계층·기존 비동기 경로 원칙을 유지한다.
