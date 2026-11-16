# 구현 계획

## 기존 구조·기술·호출 흐름
Rust/GPUI, git2 0.21, 기존 repository backend 경계와 인증 콜백을 사용한다. Pull은 SyncRequest의 백그라운드 작업 및 Project/PreparedHistory 재조회를 갖고 있어 같은 갱신 패턴을 재사용한다. 공용 Dialog/form_controls와 타이틀바 제외 ProgressOverlay를 사용한다. toolbar는 공통 아이콘/하단 문구의 작업 버튼 부분을 재사용한다.

## 입력 설계
기존 TextInput의 모델·EntityInputHandler·UTF-16 변환·선택·클립보드·포커스 처리를 유지하고 multiline 모드를 추가한다. 단일 줄 정규화/레이아웃은 기존대로 두며 TextAreaElement가 줄별 ShapedLine·선택/조합 밑줄·커서와 GPUI ScrollHandle을 제공한다. 편집 알고리즘은 원문 교체를 공통 함수로 분리하고 모드별 정규화만 달리한다. 다중 줄 키 컨텍스트에서만 Enter/위아래/문서 이동을 바인딩하여 기존 저장소 팝업 Enter 확인을 가로채지 않는다. 긴 메시지는 스크롤하고 편집 시 커서를 보이게 한다.

## Git 설계
repository 하위 committing.rs에 읽기 전용 CommitDraft 준비와 CommitRequest/CommitOutcome을 추가한다. draft는 현재 참조·HEAD·기존 메시지·스테이지 여부를 제공한다. 확인 시 HEAD/참조가 그대로인지 검사하고 index.lock을 확보해 인덱스만 커밋한다. 일반·초기·정정·해결된 merge 부모를 처리하고 미해결 충돌/다른 진행 상태는 거부한다. amend는 원저자와 부모를 유지한다. 푸시는 기존 credentials 콜백과 강제 표시 없는 refspec을 사용한다. 원격 거부 콜백도 확인하고 부분 성공을 별도 결과로 전달한다.

## UI와 상태
CommitDialog는 TextInput multiline 엔티티와 일반/정정 초안, 옵션, 읽기/실행/오류 상태를 갖는다. Draft는 백그라운드로 읽는다. 확인 이벤트만 Workspace에 CommitRequest를 전달한다. Workspace는 별도 요청 세대·작업 가드, 전역 진행 포커스를 관리하고 결과와 별개로 스냅샷을 재조회한다. 커밋 실패는 팝업 입력을 보존하고, 커밋 성공은 푸시 오류 여부와 관계없이 팝업을 닫아 중복 커밋을 막는다. Git 상태 갱신 helper를 Pull과 Commit이 공유한다.

## 영향 및 불변 조건
text_input.rs, form_controls.rs, commit_dialog.rs, committing.rs, repository.rs, backend.rs, workspace_backend.rs, ui.rs, split_button.rs, main.rs를 변경한다. 헌법은 미작성 템플릿이며 계층 분리·호환성·최소 변경을 유지한다. 새 의존성·전역 설정 변경·자동 스테이지·강제 푸시는 없다.

## 검증
다중 줄 정규화/Unicode·UTF-16 편집·선택 교체·줄 이동, 기존 단일 줄/IME/비밀번호 테스트를 검증한다. 임시 저장소와 로컬 bare 원격으로 staged-only·첫 커밋·일반·amend·병합 부모·충돌/잠금/빈 메시지/HEAD 변경·푸시 성공 및 비 fast-forward 거부/네트워크 오류의 부분 성공을 검증한다. 상태 결과 적용·중복 방지와 전체 테스트·fmt·엄격한 clippy·별도 빌드를 실행한다. 실제 사용자 저장소에서는 실행하지 않는다.

진행 중에는 CommitDialog 엔티티와 초안을 보관하되 렌더를 잠시 숨겨 전역 진행 창 아래에 다른 전체 창 배경막이 겹치지 않게 한다. 기존 타이틀바 제외 진행 표시를 유지하며 실패 시 같은 팝업을 다시 보여준다. CommitOutcome의 후속 오류 필드는 push 또는 병합 상태 정리 실패를 포함하여 생성된 커밋 id와 구분한다.
