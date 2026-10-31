# 데이터 모델

- Workspace: WorkspaceTab 배열, 활성 탭 ID, 다음 고유 ID, 전역 오류.
- WorkspaceTab: Empty(고유 ID) 또는 Repository(Project). 빈 탭에는 경로와 저장소 조회 상태가 없다. 두 종류는 같은 ID 발급 규칙을 사용한다.
- SplitLayout: 열 비율과 행 비율(초기 0.3과 0.5). 본문 가용 크기에 최소 크기를 적용한 유효 비율을 계산한다.
- Workspace 크기 조절 상태: 본문 Bounds와 활성 드래그 축(열/행/없음). 마우스 해제 시 드래그 축을 비우고 프로젝트 전환에서는 비율을 유지한다.
- Project: ID, 정규화 경로, 표시 이름, 참조 탭, 파일 탭, 선택 참조/커밋/파일, 조회 결과, 로딩, 오류, 요청 세대.
- Reference: 종류(Local/Remote/Stash/Tag), 이름, 대상 커밋. Stash는 reflog 각 항목이다.
- Commit: 식별자, 제목, 작성자, 시각. 선택 참조의 도달 가능한 최대 200개다.
- FileChange: 원본 경로 바이트, 표시 경로, 변경 종류, 비교 출처, 이전/이후 객체와 종류.
- Diff: 제목, 행(문맥/추가/삭제/헤더), 크기 제한 또는 바이너리 안내.

## 상태 전이

프로젝트 전환은 활성 ID만 바꾸고 선택을 보존한다. 참조 변경은 커밋 및 History 파일 선택을 초기화한다. 커밋 변경은 History로 전환하고 파일 선택을 초기화한다. 파일 탭 변경은 차이를 비운다. 새로고침은 유효한 참조를 유지하고 사라진 참조는 HEAD로 복귀한다. 응답은 프로젝트 ID와 요청 세대가 일치할 때만 적용한다.

+를 누르면 고유 ID의 Empty를 추가하고 활성화한다. 탭 닫기는 해당 항목만 제거하며 활성 탭이 사라지면 남은 탭을 선택한다. 마지막 탭 제거 시 Empty를 하나 만든다. 저장소 없이 실행하면 Empty로 시작한다. 저장소 열기는 실행 인자와 새 탭에서 제공하고 상단의 수동 새로고침 버튼은 제거한다.

## 새 탭 시작 상태

고유 ID별 URL·Clone 입력 표시·오류·진행 여부를 보관한다. 성공하면 같은 ID의 저장소 탭으로 전환한다. 닫힌 ID의 결과는 버린다.

## 커밋 그래프 및 상세 모델

Commit에 parent_ids와 그래프 노드·입출력 레인을 추가한다. CommitDetail은 전체 ID·부모·작성자·이메일·커미터·이메일·시간·시간대·전체 메시지를 가진다. Snapshot은 선택한 커밋의 상세 하나를 선택적으로 가진다. Project는 상세 선택과 파일 선택을 분리하며 상세 선택 중 파일 인덱스는 없다.

## 전체 이력 및 그리드 상태

Reference.target은 참조의 커밋 ID다. Commit은 해당 커밋을 가리키는 로컬·원격 브랜치명 목록과 커미터 시간대 오프셋을 가진다. 선택 조회 결과는 파일 목록과 CommitDetail만 담는다. UI는 프로젝트별 가상 목록 스크롤 핸들과 가시 행 캐시, 창 공통 열 비율·측정 폭·열 드래그 상태를 가진다. 전체 조회만 캐시를 비우고 선택 조회는 이력을 유지한다.

## 페이지 및 목록 상태

CommitPage는 최대 200개 commits와 has_more를 가진다. HistorySession은 요청 채널 핸들이며 작업자는 저장소·위상 iterator·레인 상태를 소유한다. Snapshot은 현재 브랜치 전체 참조와 종류별 참조 인덱스를 가진다. Project는 초기 조회와 독립적인 페이지 진행/오류/세대를 유지한다. 스크롤은 프로젝트·목록별 핸들과 트랙 Bounds, 전역 드래그 상태를 가진다. Stage 가상 행은 그룹 또는 원래 파일 인덱스다.

## 조건부 스크롤 상태

목록별 프레임 Bounds와 고유 콘텐츠 크기에서 축별 표시 여부를 계산한다. Diff와 상세의 측정된 텍스트 폭은 데이터 변경 때만 무효화한다. 상세 문자열 캐시는 상위 항목별 한 줄과 시간대 없는 로컬 시각, 원본 메시지 행을 포함한다.

## 날짜·해시 기본 너비

열 배치 상태의 기본 가중치에서 Date와 Hash는 0이다. 기존 수동 경계 조절은 가중치를 갱신하며 데이터 모델과 저장소 조회는 그대로 사용한다.

## 영속 세션과 모달 상태

세션은 버전·탭별 선택적 경로·활성 인덱스·최근 경로 목록을 가진다. 탭 ID는 실행 중 재발급하며 복구 순서와 활성 인덱스를 보존한다. 최근 목록은 경로가 고유 키이며 열 때 맨 앞으로 이동한다. 진행 상태는 작업 카운터와 기존 프로젝트 조회/페이지/Diff 및 시작 작업 상태의 합성값이다. 삭제 확인 대상·도움말 열림은 일시 상태이다.

읽기 전용 텍스트 선택은 행과 Unicode 문자 위치를 저장하고 실제 글꼴 배치로 화면 위치를 매핑한다. 복사 시 표시용으로 펼친 탭 대신 원문을 사용한다. 포커스한 목록 종류는 브랜치·파일·히스토리·텍스트 중 하나이며 모달과 진행 상태가 입력보다 우선한다.

히스토리 진행 상태는 Project.history_loading과 page_loading이다. history_target은 위치를 찾는 브랜치 커밋 ID이다. 발견/끝에서 해제하고 중간 오류에서는 재시도를 위해 유지한다. 재조회와 새 커밋 선택은 이전 대상을 초기화한다.


Stage 표시 문자열은 FileChange.status와 staged에서 파생하며 원래 변경 종류와 Diff 전후 객체는 유지한다. 파일 행 매핑은 staged 그룹 헤더만 포함하고 unstaged 그룹 헤더는 만들지 않는다. Workspace의 저장소 ID별 staging 집합은 비동기 인덱스 쓰기 중복을 차단한다. 원격 참조 이름과 revision은 peel 이전 값을 유지한다.


Stage의 UI 행은 그룹 없는 파일 인덱스 매핑이다. 인덱스 작업은 요청 당시 staged 여부의 반대 값을 목표 상태로 저장하고, 완료 시 현재 선택 경로가 같은 경우 목표 상태 행을 선택한다. 영속 데이터 구조 변경은 없다.


Project의 최초 조회 플래그는 탭 생성 시 false이고 reload 시작 시 true이다. 실패 후 재조회는 F5로 명시적으로 수행한다. 커밋 모달은 대상 ID·입력 Entity·실행 여부·오류를 가지며 영속 세션에는 저장하지 않는다. 입력기는 UTF-8 텍스트와 바이트 선택/조합 범위를 저장하고 OS 입력에는 UTF-16으로 변환한다. Stage 정렬은 경로가 기본 키이다.


커밋 입력기에 가시 영역 크기와 스크롤 축별 드래그 시작 위치를 추가한다. 제출 여부는 메시지와 별도로 staged 변경 존재를 확인한다. 영속 상태 변경은 없다.


Commit.author_email은 작성자의 메일이며 author와 분리한다. StageSelection의 키는 경로와 staged 여부이고 범위 기준도 같은 키이다. 대표 selected_file은 Diff 조회에만 사용하며 실제 Space 작업은 선택 집합 전체를 사용한다. 상태별 경로 요청은 중복 제거 후 실행하고 최신 행에 선택을 다시 매핑한다.

폐기 모달은 선택한 고유 경로와 프로젝트 ID를 캡처하고 실행/오류 상태를 가진다. 대상은 HEAD 항목 유무로 복원/신규 삭제로 나눈다. 선택 복원은 실제 재조회 결과에 남은 행만 보존한다.

원격 작업의 종류는 fetch/pull/push이며 Workspace는 진행 중 저장소 ID와 표시 라벨을 일시적으로 보관한다. 영속 세션 변경은 없다.


StageSelection 전체 선택은 현재 파일들의 경로/상태 키를 모두 저장한다. 기존 기준 키가 목록에 있으면 유지하고 없으면 첫 행을 기준으로 둔다. 폐기 모달의 내부 대상 경로는 표시 목록 제거와 무관하게 유지한다.

하단 오류 표시에는 마지막 관찰 문자열과 현재 표시 문자열, 타이머 세대를 둔다. 3초 만료는 표시만 지우고 조회 실패/모달 상태를 변경하지 않는다.

## 선택 행 표시 상태

선택 여부와 키보드 포커스는 독립이다. 활성 저장소의 ListFocus가 해당 목록이고 워크스페이스가 실제 포커스를 가지며 차단 모달/히스토리 작업이 없을 때 활성 선택이다. 그 외 선택은 비활성 선택이다. 상태는 저장하지 않고 렌더링할 때 계산한다.

## 최근 목록 화면 상태

StartPage.recent_scroll은 새 탭별 가상 목록 스크롤 핸들이다. recent는 기존 PathBuf 순서 목록을 그대로 사용한다. delete_target이 없으면 닫힘, 대상이 있고 delete_confirm=false이면 동작 선택, true이면 최종 삭제 확인이다. 목록 제거는 recent만 갱신하고 세션으로 저장한다.

커밋상세 행의 선택은 기존 show_commit_detail이며 포커스는 ListFocus::Files이다. 새 상태 필드는 추가하지 않는다.

빈 탭 전환 입력은 시작 탭 ID와 정규화된 저장소 경로이다. 성공 출력은 결과 프로젝트 ID이며 없어진/전환된 시작 탭에는 결과가 없다. 미개방 저장소는 기존 ID를 유지하고 이미 열린 저장소는 기존 프로젝트 ID를 유지한다. StartPage.busy는 비동기 최근 열기에도 적용한다.

도움말 섹션은 제목과 키별 표시 문자열 목록이며 영속 상태나 런타임 키 바인딩 모델을 추가하지 않는다. 기존 도움말 표시 상태를 유지한다.

## 설정 모델

- GitContext: 열 때 캡처한 선택 저장소 또는 전역 전용 컨텍스트. 테스트에서만 격리 global 파일/환경을 사용한다.
- Identity: name/email. SettingsSnapshot: 전역·개별·유효 Identity, 원격 정보 목록, 브랜치/추적, GCM 설치/연결 정보.
- RemoteInfo: name, fetch_urls, push_urls. URL 표시 시 민감 구성요소를 제거한다.
- SettingsPanel: 사용자/원격 활성 탭, 이름/이메일/로그인 URL 입력 엔티티, 전역 여부, 조회/저장/인증 진행 상태, 오류/성공 안내, 취소 플래그. 패널은 새로 열면 다시 읽는다.
- LoginTarget: 검증된 비밀 없는 HTTPS URL에서 만든 protocol/host/path. CredentialState: 미확인/연결됨(계정명)/로그인 필요. 비밀번호·토큰은 상태 모델과 세션에 포함하지 않는다.

## 브랜치와 태그 상태

CheckoutTarget은 local 이름과 선택적인 remote 전체 참조, 생성 필요 여부를 갖는다. BranchDialog는 프로젝트 ID·경로·생성 커밋/원격추적 대상·선택적 이름 입력·실행 상태·오류를 갖는다. 생성 커밋은 열 때 캡처한 객체 ID이며 확인 전에는 참조를 바꾸지 않는다. Commit.tags는 해당 커밋을 가리키는 태그 이름 목록이고 기존 branches와 분리한다.

## 스태시 확인 상태

StashDialog는 project_id, busy, error를 가진다. 대기 → 실행 → 성공 시 닫기 또는 오류 후 재시도로 전환한다. 영속 데이터는 Git stash이며 세션 필드는 추가하지 않는다.

## 스태시 복원 상태

Reference의 Stash revision과 target은 커밋 해시다. 복원 작업은 저장소 ID와 선택 해시를 캡처한다. branch_busy·pending은 실행 중에만 유지하고 별도 영속 필드는 없다.

## 저장·삭제 확인 상태 확장

StashDialog는 기존 project_id·busy·error 외에 선택적 editor와 선택적 삭제 대상(selector, commit, name)을 가진다. 저장에서만 editor가 존재하고 삭제에서는 대상만 존재한다. 실행 중 editor.enabled를 끄고 실패 시 다시 켠다.

## 참조 작업 상태 확장

BranchAction에 Tag(commit)와 Delete(reference, expected)를 추가한다. 이름 입력은 Create·Tag, 확인만 필요한 Delete는 editor가 없다. 기존 BranchDialog와 branch_busy로 작업을 격리한다.

Snapshot.head_commit은 Option<String>으로 unborn이면 None, 일반/Detached HEAD에서는 실제 커밋 해시다.

## 원격 생성 상태

BranchAction::Upload는 Local 전체 참조와 예상 커밋을 보관하고 editor는 원격 이름을 입력한다. 커밋 체크아웃은 저장소 ID와 커밋 해시를 캡처하고 기존 branch_busy·pending으로 진행을 격리한다.

## 커밋 체크아웃 후보

CheckoutCandidate는 reference와 label을 가진다. BranchAction::ChooseCheckout은 commit, candidates, selected를 가진다. 선택은 팝업의 임시 상태이며 저장소에는 쓰지 않는다. 확인 시 commit과 ref tip을 재검증한다.

## 즉시 선택 상태

ChooseCheckout.selected는 클릭한 인덱스를 실행 함수로 전달하는 임시 필드로 유지한다. 기본 선택은 Enter로 제출하지 않으며 클릭만 실행을 시작한다.

## 팝업 표시 인수

공통 프레임은 title·width·body·footer·header_action·viewport를 입력받는다. footer/header_action은 선택적 UI 요소이고 상태나 영속 데이터는 추가하지 않는다.

## 무변경 체크아웃 결과

히스토리 준비 결과에 switched 여부를 함께 전달해 후보 없음/실제 전환과 현재 브랜치 무변경을 구분한다. 영속 상태 추가는 없다.

## 테마 표시 상수

theme 모듈의 의미별 u32 색상과 그래프 팔레트를 사용한다. 모델·저장소·세션 데이터와 영속 설정은 추가하지 않는다.

## 그래프 레이아웃

렌더링 인수 graph_lanes는 현재 로드된 전체 커밋의 marker/input/output 레인 최댓값+1이다. 경로 캐시 키는 원점·폭·레인 수다. 영속 데이터는 없다.

## 상세 표시 범위

TextRows는 첫 빈 줄 이전을 메타데이터 구간으로 판별한다. 표시 run은 라벨 바이트 길이를 기준으로 굵은 라벨/일반 값을 분리한다. 문자열과 영속 데이터 구조는 유지한다.

## 표시 상수 보완

Local 배지 배경 토큰과 HEAD 반지름·목록 아이콘 크기만 바꾸며 데이터 모델은 유지한다.

## 배지 원 표시

기존 current_branch 비교를 사용하며 데이터 모델은 변경하지 않는다.

## 고정 원 표시

기존 current_branch 판별을 유지하고 크기와 중앙 위치만 UI helper에서 관리한다.

## 원격 HEAD 표시 판별

Reference의 Remote 종류와 /HEAD 이름으로 표시 제외를 판별한다. 원본 데이터와 영속 상태는 유지한다.

## 그래프 캐시 DPI

경로 캐시 키에 화면 scale_factor를 추가한다. 영속 모델은 변경하지 않는다.

## 상세 렌더 레이아웃

줄바꿈 없는 WrappedLine과 unwrapped_layout을 사용해 paint/선택좌표/폭 측정을 통일한다. 기존 텍스트 데이터는 유지한다.

## 드래그 상태

기존 text_drag·text_selection·viewports·list_scroll을 재사용하며 영속 데이터는 추가하지 않는다.

## 단축키 표시

공통 표시 helper만 추가하며 저장 데이터와 실제 키 바인딩은 변경하지 않는다.

## 생성 선택 상태

Create action에 remote bool을 추가하고 편집기에 개별 text_size를 추가한다. 영속 데이터는 변경하지 않는다.

## 생성 옵션

Create의 checkout bool은 기본 false다. 체크박스는 표시 helper이며 입력 bounds는 세로 오프셋을 반영한다.

## 체크 여백

표시 레이아웃만 바꾸고 상태/데이터는 유지한다.

## 입력 포커스

브랜치 체크용 FocusHandle 두 개와 설정 전역 체크 FocusHandle을 추가한다. 영속 데이터는 유지한다.

## 체크 상태

기존 상태/포커스/클릭 동작을 유지하고 입력 경로만 단일화한다.

## 숫자 키 매핑

기존 ListFocus만 사용하고 영속 데이터는 변경하지 않는다.

## 팝업 옵션

PopupOptions는 branch_remote·branch_checkout·use_global(선택적 bool)을 가지며 serde default로 과거 세션을 지원한다.

## 포커스 복원 상태

일회성 focus_after_popup 플래그만 추가하고 기존 list_focus/revision을 재사용한다.

## 후보·포커스

기존 list_focus와 ref_tab을 유지하고 후보 label 형식만 바꾼다.

## Reset 상태

ResetMode는 Soft/Mixed/Hard이며 Reset action은 대상 commit·mode·selectbox expanded를 가진다. 별도 영속 데이터는 없다.


## 히스토리 작업 상태

Reset은 커밋/모드만 유지한다. Revert는 커밋/no_commit을 가진다. Integrate는 Merge/Rebase 종류, 독립 커밋 목록, 선택 인덱스, 조회 상태를 가진다. PopupOptions.revert_no_commit은 기본 false이고 세션에 저장된다.


## Merge·Rebase 고정 대상

Integrate는 kind와 commit만 보유한다. 이전의 commits/selected/loading과 전용 스크롤·조회 세대는 제거한다.


## 단축키 설정 데이터

기능 정의는 안정 ID·영역·이름·기존 명령·기본 두 키이다. 사용자 설정은 기능 ID에서 두 문자열 슬롯으로의 맵이다. 빈 문자열은 해제를 뜻하며 미등록 ID는 실행하지 않는다. 같은 영역 또는 공통 영역의 기존 할당과 충돌하면 거절한다. 화면 상태는 열림·등록 대상·안내 문구이며 영속화하지 않는다.


## 확인창 공통 설정

confirm, cancel, next-input, previous-input 네 ID가 두 키 슬롯을 공유한다. dialog 범위는 기존 확인창 범위와 주소 입력창에 적용한다. 과거 창별 submit/close/tab/backtab ID는 더 이상 정의되지 않아 무시된다.


## 커밋 옵션 상태

CommitDialog.push_after는 팝업 동안 유지되는 체크 상태다. 초기 false이고 실행 중 고정된다. 실패 시 메시지와 함께 유지한다. 별도 세션 저장은 추가하지 않는다.


## 설정 선택 탭

선택 탭은 GROUPS의 인덱스이며 초기값 0인 임시 화면 상태다. 열기 시 초기화하고 세션에는 저장하지 않는다. 세 기능의 ID는 유지하고 name만 교체한다.


## 단축키 임시 편집 상태

활성 shortcuts는 실제 실행·도움말·일반 세션 저장에 사용한다. shortcut_draft는 팝업 열기 시 활성 설정을 복제한 Option 값이다. 탭 전환은 같은 복제본을 사용한다. 확인 성공 또는 취소 시 None으로 해제한다. 저장 실패는 복제본을 유지한다.


## 공통 방향·전환 정의

nav-up/down/left/right ID는 유지하고 navigation 범위를 사용한다. input-toggle은 branch/reset/commit-modal의 공유 두 슬롯이며 기본 Space다. reset 방향·창별 Space 정의는 제거되어 저장된 과거 ID는 무시한다. branch-jump/files-stage와 설정 승인 데이터 모델은 유지한다.


## Stage 파일 단축키 범위

stage-files는 활성 저장소의 Stage 파일목록에만 해당한다. files 범위의 키와 주 화면 공통/방향키를 상속한다. stash/commit은 해당 범위에만 정의하고 저장 ID와 두 슬롯은 유지한다.


## 하위 분류 표시 데이터

브랜치 하위 분류는 Local/Remote/Stash/Tag, 파일목록은 Stage/History다. Command는 복제 저장하지 않고 분류별 포함 판정으로 재사용한다. UI 요소 ID에는 분류명을 추가하여 중복을 피하고 Bindings ID는 유지한다. 가시성은 선택 확장·checkout/reset 전용 정의를 제외한다.


## 삭제 체크 상태

기존 BranchAction::Delete.server를 팝업 내 체크 상태로 사용한다. 참조 경로가 refs/remotes/일 때만 변경 가능하다. 세션에 저장하지 않고 취소 시 팝업 상태와 함께 폐기한다.


## 연결 삭제 계획

삭제 계획은 Local/Remote/Tag 종류, 주 대상 참조·기대 해시·선택 원격 서버/목적지, 연결 대상 배열을 보관한다. 원격 목적지는 fetch refspec으로 확정한다. 계획은 비교 가능하며 추가 삭제 시 관계까지 다시 비교한다. UI는 계획과 delete_related만 보관하고 초기 false다. Stash는 기존 reflog 예상 해시를 유지한다. 세션 저장은 추가하지 않는다.


## 브랜치 키 우선순위

branch-upload의 저장 ID는 유지하고 기본 슬롯만 shift-p로 변경한다. local-branches는 실제 활성 저장소 Local 참조 포커스다. 정의의 group이 브랜치이면 공통보다 높은 해석 우선순위를 갖는다. 우선순위는 저장 설정에 새 필드를 추가하지 않는다.
## 팝업 입력 드래그 상태 (US86)

새 영속 데이터나 개체는 없다. 공통 카드의 내부 누름이 배경의 GPUI 클릭 대기 상태로 전이되지 않게 한다. 내부 입력 선택과 포커스 상태는 기존 모델을 따른다.

## 커밋 줄바꿈 상태 (US87)

새 개체나 저장 데이터는 없다. 기존 커밋 대화상자의 진행 상태·편집기 포커스와 선택 영역을 사용한다.

## 파일 트리 상태 (US88)

FileRow는 파일 원본 인덱스 또는 폴더 원시 경로와 접힘 상태를 표현한다. 실행 중 공통 bool 모드와 프로젝트/Stage·History 키별 접힌 경로 집합을 UI에서 보관한다. 가시 인덱스 배열은 파일 선택/탐색 순서를 정의하며 실제 파일 데이터와 선택 키는 유지한다.

## 폴더 작업 상태 (US89)

US88의 접힘 필드/집합을 제거한다. 폴더 행은 원시 경로만 가진다. UI의 선택 폴더는 목록 키와 원시 경로의 선택값이며 파일 선택과 상호 배타적이다. 폴더 작업 맵은 하위 원시 파일 경로별 한 항목과 공통 목표 Stage bool을 가진다.

## upstream 연결 계획 (US90)

UpstreamPlan은 Local 전체 참조·예상 해시·현재 upstream·후보 배열을 가진다. UpstreamCandidate는 전체 원격 추적 참조·표시 이름·예상 해시와 원격명/서버 브랜치 매핑을 보관한다. UI Connect는 계획과 선택 인덱스를 가진다. 새 영속 포맷 없이 Git의 기존 upstream 설정을 사용한다.

## 라디오 포커스 (US91)

기존 Connect.selected와 후보 배열을 유지하며 Workspace에 라디오 그룹 FocusHandle만 추가한다. Local 표시 삭제는 UpstreamPlan.reference의 실행 대상 데이터에 영향을 주지 않는다.

## Merge 복구 상태 (US92)

MergeState는 root, marker_modified, head, current_branch, merge_heads, conflicts를 가진다. 같은 작업 판정은 정규화한 저장소 경로/MERGE_HEAD 수정 시각/HEAD/현재 브랜치/병합 대상의 일치로 수행하며 conflicts는 Stage 후 변할 수 있다. Snapshot.merge는 진행 중일 때만 존재한다. BranchAction의 MergeControl은 확인 시점의 MergeState와 완료/중단 종류를 보관한다. Unmerged 파일은 stage=false/status=U로 한 번만 나타난다.

## Rebase 상태 (US93)

RebaseState는 정규화한 root·backend·orig-head 수정 시각·원래 HEAD/브랜치/브랜치 끝·onto·현재 HEAD·현재 커밋·단계/전체·미해결 경로를 가진다. same_session은 저장소/방식/원래 대상/기반을, same_operation은 현재 단계까지 비교한다. Stage 후 conflicts만 갱신 가능하다. Snapshot.rebase와 merge는 구분하며 공통 진행 여부를 UI 가드에서 사용한다. BranchAction.RebaseControl은 확인 시점 상태와 Continue/Abort/Skip을 보관한다.

## Revert 상태 (US94)

RevertState는 root·현재 head/branch·sequence_head·세션 시각·현재 대상/대상 시각·todo 원문/대기 개수·opts 원문·선택적 no_commit·충돌 경로를 가진다. 같은 세션과 같은 단계 판정은 원시 상태를 비교하며 Stage로 conflicts만 바뀔 수 있다. Snapshot.revert를 추가한다. 확인 액션은 상태와 Continue/Abort/Skip 및 옵션 미상 시의 no_commit 선택을 보관한다.

## Stash 복구 상태 (US95)

StashRestoreState는 세션 ID/저장소/원본 Stash/HEAD/브랜치·개정/적용 중 여부/실패 안내·미병합 경로를 가진다. 전용 메타데이터는 스키마 버전·경로별 원본 유무/권한/해시·원본 인덱스 해시·인덱스 항목·변경 경로·부모 디렉터리 상태를 기록한다. 경로는 원시 바이트, 백업 이름은 숫자 인덱스만 사용한다. Snapshot.stash_restore와 unmerged를 추가한다. 완료/취소는 세션 ID/개정과 실제 저장소 상태를 검증한다.

원본 Stage 객체는 복구 전용 Git 참조에 트리로 고정해 정리 작업에도 보존한다. 원본 인덱스와 별도로 분할 인덱스를 해제한 전체 사본을 보관하며, 공유 인덱스가 사라지면 검증한 전체 사본으로 복구한다. Windows Git에 전달하는 인덱스 경로는 확장 경로 접두사를 제거한다.

## US96 충돌 구간

충돌 구간은 시작 줄·전체 바이트 범위·현재 내용 범위·들어오는 내용 범위로 구성한다. 충돌 미리보기는 원문과 구간 목록을 보관한다. 선택은 현재/들어오는/양쪽이며 양쪽 순서는 현재 다음 들어오는 내용이다. 하나 적용 후 원문이 바뀌므로 모든 구간을 재조회한다.

## US97 표시 행·부분 Stage

충돌 동작 행은 충돌 구간 번호만 보관하고 원문 좌표는 없다. 바로 다음 행은 원래 마커와 좌표다. 부분 Stage 미리보기는 인덱스 이전 원문·작업 파일 이후 원문을 보관한다. 줄 선택은 프로젝트 ID/미리보기 세대에 귀속된 표시 행 집합이며 추가/삭제만 유효하다. 재조회·파일 전환으로 세대가 바뀌면 선택을 폐기한다. 구간은 헤더 다음부터 다음 헤더 직전까지의 추가/삭제 행 집합이다.

## US98 커서와 충돌 구간

충돌 구간은 시작 줄과 종료 줄을 포함한다. 현재 대상은 Diff 표시 행의 작업 파일 new 줄 번호로 구간을 조회한 결과이며 별도 저장하지 않는다. 좌표 없는 삭제/헤더 줄은 대상 없음이다. 합성 충돌 동작 행과 관련 필드는 제거한다. 커서는 기존 프로젝트별 Selection.cursor를 사용하고 파일/미리보기 변경 시 초기화한다.

## US99 버튼 단축키 안내

기존 Bindings의 명령 ID와 두 슬롯에서 렌더링 시 안내를 파생한다. 별도 저장 상태는 없으며 빈 슬롯은 표시하지 않는다.

## US100 오류 기록

오류 기록은 출처 키·표시 출처·전체 메시지의 순서 목록과 출처별 관찰 상태, 현재 페이지를 가진다. 같은 출처/메시지는 유지 동안 한 번만 추가하고 해제 후 재발생은 다시 추가한다. 팝업 열림 여부는 Workspace가 보관한다. 앱 종료 시 모두 소멸한다.

## US101 확인완료 전이

현재 오류 제거 후 동일 인덱스의 다음 항목을 선택하고 인덱스가 끝을 넘으면 새 마지막 항목으로 보정한다. 빈 기록은 0을 유지한다. 마지막 관찰 상태는 제거하지 않는다. open은 마지막 인덱스로 이동한다.

## US102 명령 변경

오류 기록 명령의 기본 슬롯은 F10/빈 값이며 전역 범위를 가진다. 확인 기본 슬롯은 Enter/빈 값이다. 저장된 사용자 재지정은 기존 Bindings 규칙을 따른다. 오류 수명/순서는 변경하지 않는다.

## US103 범위와 유효 행

기존 Selection의 정규화된 시작/끝에서 표시 행 범위를 파생한다. DiffLine의 old 또는 new가 있는 행 인덱스를 유효 커서 행으로 사용한다. 체크 집합은 기존 프로젝트/세대별 BTreeSet을 유지한다.

## US104 표시 상태

새 데이터 상태는 없다. 기존 오류 건수에서 표시 내용/너비를 파생하며 기록·페이지·명령은 유지한다.

## US105 상태 보존

배치·색·패딩만 변경하고 오류 기록·페이지·단축키 상태는 유지한다.

## US106 오류 발생 단위

명시적 report는 출처 키·표시 출처·메시지를 받아 목록에 추가하고 observed를 갱신한다. 설정은 오류 대입 버전을 별도로 가진다. 현재 페이지/메시지와 읽기 전용 편집기 캐시는 Workspace에만 존재한다. 확인완료는 기존처럼 목록만 제거한다.

## US107 상태 보존

기본 배경 알파만 변경하며 오류 기록·선택·페이지·단축키 모델은 유지한다.

## US108 표시 행 매핑

표시 행은 원문 시작 바이트와 ShapedLine을 가진다. 줄바꿈 문자나 원문 버퍼는 추가/수정하지 않는다. 읽기 전용 모드에서 뷰포트 폭으로 재계산한다.

## US109 오류 전달 모델

report는 새로운 발생으로 한 건 추가하고 mirror는 동일 발생을 표시하는 출처의 observed만 갱신한다. 충돌 파일은 기존 RevertState.conflicts 원시 경로를 사용한다.

## US110 명령과 상태

기존 RefTab/FileTab과 목록 포커스를 재사용한다. 영역별 이전/다음 네 명령의 기본 슬롯은 [/빈 값 또는 ]/빈 값이다. 추가 영속 상태는 없다.

## US111 입력 분류

기존 ClickEvent의 Mouse/Keyboard와 KeyDownEvent.is_held를 사용한다. 체크 상태/단축키 저장 모델은 변경하지 않는다.

## US112 명령/상태

설정 탭/프로젝트 탭 이전·다음 네 명령을 추가한다. shortcuts-modal은 대화상자 범위이고 project-tabs는 일반 화면 범위다. 기존 shortcut_tab·WorkspaceTab·Bindings와 원격 내부 target을 재사용한다.

US112 공통화 보완: 내부 tab-previous/next는 하나의 Bindings 설정을 브랜치/파일/설정 팝업이 공유한다. 프로젝트 명령 두 개는 별도 공통 항목이다.

## US113 기준 명령과 이전 ID

files-all의 공유 범위는 files/stage-files/viewer이고 branch-checkout의 공유 범위는 branches/local-branches/history다. 이전 viewer-all/history-checkout은 저장 값 읽기 승계에만 사용한다. 기준 ID가 비어 있는 슬롯 배열을 명시한 경우도 기준 값을 우선한다.

## US120 상태 변경

세팅의 탭/전역 선택 상태를 제거하고 명시적 저장 대상 및 세 버튼 포커스를 사용한다. 이름/이메일 draft와 원격 인증 URL/계정은 독립이다. snapshot은 global/effective/info만 유지한다. 세션의 옛 use_global은 읽기 호환 필드로 남지만 동작에 사용하지 않는다.

## US121 명령 변경

tab-previous/next 활성 명령을 제거하며 과거 override 데이터는 방향 설정에 합치지 않는다. nav-left/right가 horizontal-navigation scope에서 탐색과 내부 탭을 담당한다. project-tab-previous/next는 독립 명령/두 슬롯을 유지한다.

## US122 작성자 상태

use_global은 실제 local override 유무로 초기화하며 옛 세션 선호값을 사용하지 않는다. local_draft는 체크 전 편집값을 보존한다. snapshot에 global/effective/use_global/info를 제공하고 두 포커스는 체크와 저장에 대응한다. US120의 세 버튼 상태를 대체한다.

## US127 실행 모델

UI 모델은 유지한다. backend는 gix Repository/Config/Index/Object/Ref를 우선 사용하고 git2 Repository/Remote/통합 API를 복합 작업 단위로 열어 인프로세스 결과를 반환한다. 통합/복구 상태는 기존 Git 상태와 비교 가능한 형태로 저장한다. 인증 정보는 메모리 콜백 내에서만 전달한다.

## US128 저장소 설정 일관성

새 영속 데이터는 없다. 조회와 쓰기는 같은 시스템→전역→저장소 설정 우선순위를 사용하며 조회만 외부 필터 실행을 차단한다.

## US129 스태시 추가 표시

기존 StashDialog의 editor·deletion·error·busy 상태를 그대로 사용하며 데이터 구조 변경은 없다.

## US130 참조와 그래프 범위

Reference의 Stash 분류와 목록을 유지하되 일반 그래프 tips에서 제외한다. 선택한 스태시의 CommitSelection은 그래프 포함 여부와 독립적이다.

## US131 읽기 전용 스타일

기존 read_only 상태를 그대로 사용하며 추가 데이터나 저장 설정은 없다.

## US132 배경색 공유

기존 read_only 상태로 배경을 산출한다. 별도 설정·영속 데이터는 추가하지 않는다.

## US133 포커스 상태

SettingsPanel은 전역 설정 체크박스의 FocusHandle 하나만 보유한다. 저장 버튼의 포커스 상태를 제거하며 영속 데이터는 변경하지 않는다.

## US134 원격 추가 상태

SettingsPanel은 추가 버튼 핸들, 이름·URL 편집기, 팝업 열림 상태를 가진다. 저장 중에는 기존 Work::Saving을 사용한다. RemoteInfo의 이름·fetch_urls를 목록에 사용하며 영속 형식은 remote.<이름>.url과 fetch다.

## US135 팝업 상태

기존 remote_dialog·입력 편집기·작업 상태를 그대로 사용한다. 새 영속 데이터 없이 세팅 본문과 추가 팝업이 같은 SettingsPanel 상태를 공유한다.

## US136 레이아웃 보완

기존 입력 엔티티와 팝업 상태를 유지하며 데이터 모델 변경은 없다.

## US137 삭제 상태

기존 원격 이름과 Work::Saving을 재사용한다. 삭제 결과는 RepositoryInfo만 교체하고 Identity 초안은 유지한다. 신규 영속 데이터는 없다.

## US138 원격별 포커스

SettingsPanel은 원격 이름을 키로 하는 FocusHandle 맵을 보유한다. 목록과 함께 동기화하고 기존 원격의 핸들을 유지한다. 영속 데이터는 변경하지 않는다.

## US139 진단 데이터

일시적인 오류에는 원격 이름·git2 오류 분류/코드·가림 처리한 원문을 포함한다. 저장소 설정과 영속 스키마는 변경하지 않는다.

## US140 인증 모델

AuthMethod는 Gcm·BrowserGcm·BrowserOAuth·Password다. AuthProfile은 id·url·username·method를 가지며 비밀을 포함하지 않는다. AuthStore는 메타데이터 경로와 SecretStore를 사용하고 비밀 키는 앱 네임스페이스/profile id다. Secret은 표시/직렬화하지 않고 해제 시 내용을 지운다. 설정 UI는 인증 목록·핸들 맵·팝업 방식·URL/계정/비밀번호/Client ID 입력·작업 취소 플래그를 가진다. OAuth DeviceGrant는 일시적 code·검증 URL·만료·poll 주기를 보유한다.

## US141 가져온 인증 메타데이터

AuthProfile에 imported·hidden boolean을 serde 기본 false로 추가한다. 가져온 id는 URL·계정의 안정적인 해시이며 비밀을 포함하지 않는다. 숨긴 항목은 앱 파일에 남아 다음 자동 병합에서 재등장하지 않는다. 화면에는 활성 항목만 표시하고 명시적 앱 등록이 같은 URL/계정을 대체한다. 외부 메타데이터 입력은 target·username만 가진다.

## US142 표시 상태

AuthPanel은 all_profiles·표시 profiles·remote_urls를 보유한다. 표시 profiles는 원격별 가장 가까운 URL 범위의 합집합이다. 프로필 파일 스키마는 변경하지 않는다. 삭제/Tab 핸들은 표시 목록에만 대응한다.

## US143 로그아웃 상태

프로필의 URL·username·method로 GCM 로그아웃 대상을 지정한다. 명시적 실제 로그아웃은 imported hidden tombstone을 생성하지 않는다. 메타데이터 잠금이 선행되고 GCM 제거 성공 여부는 일시적 결과로만 보유한다.

## US144 배지 상태

기존 오류 개수와 errors 단축키 설정을 그대로 사용한다. 신규 상태나 영속 데이터는 없다.

## US145 상태 보존

오류 기록과 사용자 바인딩 구조를 유지한다. 기본 명령의 errors target/default만 f1로 바꾸며 새 상태는 없다.

## US146 기존 항목 제거 경계

ExistingCredentialStore는 target·username 메타데이터 목록과 target별 erase만 제공한다. 비밀은 인터페이스에 포함하지 않는다. 삭제 결과는 제거 건수와 재조회 성공으로 확인하며 기존 프로필 스키마는 유지한다.

## US147 GCM 요청 모델

AuthProfile.url은 검증된 표준 URL이며 .git을 보존한다. ResolvedAuth::Gcm은 username과 url을 전달한다. 프로필 스키마에 구주소 호환 필드는 추가하지 않는다. callback은 프로세스가 아닌 명령별 프로젝트 디렉터리 상태를 보유한다.

## US148 방식 표시

추가 팝업의 최상위 선택과 핸들은 두 개다. 기존 AuthMethod 값과 저장된 프로필 스키마는 유지한다.



## US149 상태 정리

AuthPanel의 browsers·client_id·code를 제거한다. 기존 영속 BrowserOAuth와 비밀은 보존한다. 신규 등록은 BrowserGcm·Password뿐이다.


## US150 상태

새 영속 상태는 없다. 기존 활성 프로젝트 경로와 단축키 바인딩을 사용한다.


## US151 실행 상태

영속 상태는 추가하지 않는다. 활성 프로젝트 경로를 실행 시 복사하고 기존 단축키 설정을 사용한다. 실행 계획은 터미널·Bash 경로와 Command 인수로 구성한다.


## US152 검색 상태

Workspace는 Search 표시 여부·입력 editor·결과·선택 인덱스·진행 세대번호를 가진다. 결과는 커밋(id/title) 또는 파일(path/content 일치 여부)이다. 파일 창은 프로젝트 경로·파일 경로·이력·선택 커밋·Diff 문자열을 별도로 가진다. 저장 형식은 바꾸지 않는다.


## US153 데이터

영속 모델 변경 없음. 검색 결과의 title은 검색어가 포함된 메시지 줄을 보여준다. Diff 창은 렌더링 시 줄 번호를 계산한다.


## US154 상태

Workspace는 파일 창 handle과 종료 구독을 가진다. FileHistoryWindow는 목록 FocusHandle과 읽기 전용 Diff editor를 가진다. 디스크 저장 형식은 바꾸지 않는다.


## US155 상태

기존 검색 세대번호로 재검색 경합을 제어한다. 별도 창은 선택·복사 가능한 Diff 편집 상태와 포커스 대상 상태를 유지한다. 영속 데이터 변경은 없다.
## US156 화면 상태

`FileHistoryWindow.selected_index`는 키보드로 선택한 커밋의 목록 위치다. `list_scroll`은 선택 항목을 표시할 스크롤 상태다. `list_width`와 `width_drag`는 창 내부 두 패널의 폭과 진행 중인 드래그를 나타낸다. 이 값들은 창 종료 시 사라지며 저장소나 설정 파일에 기록되지 않는다. Diff 읽기 전용 텍스트는 앞 13문자를 두 줄 번호 열, 다음 3문자를 변경 기호 열, 나머지를 본문으로 사용한다.
## US157 화면 상태

파일 이력 창의 `list_scroll`은 가상 목록의 스크롤 상태다. Diff 읽기 전용 편집기는 `diff_line_starts`에 각 줄 시작 바이트 위치를, `diff_max_columns`에 한 번 계산한 최대 표시 열 수를 보관한다. `visible_row_start`와 `lines`는 현재 화면 주변의 형상화된 줄 범위만 나타낸다. 전체 Diff 텍스트 버퍼는 선택·복사를 위해 보존된다. 이 상태는 창 안에서만 쓰이고 영속화하지 않는다.
## US158 화면 상태

새 상태나 저장 형식은 없다. 파일 이력 커밋 행은 기존 목록 패널의 현재 폭을 그대로 사용한다.
## US159 파일 이력 항목

`git_search::FileCommit`은 커밋 ID, Description, 작성자 이름, 작성자 이메일, 작성 시각 초, UTC 오프셋 분을 가진다. 검색 가능한 모든 브랜치·태그에서 해당 파일이 바뀐 커밋마다 하나의 항목을 반환한다. 파일 창은 `list_height`와 `height_drag`로 위쪽 목록 높이와 진행 중인 세로 드래그를 관리한다. 분할 높이는 창 종료 시 사라지고 설정 파일에 기록되지 않는다.
## US160 파일 창 단축키 상태

파일 이력 창은 생성 시점의 `shortcut::Bindings` 복사본을 가진다. 창을 닫으면 사라지는 화면 상태이며 새로운 설정 파일이나 저장 형식은 없다. 부모 Workspace의 단축키 설정이 현재 바인딩의 출처다.
## US161 Diff 단축키 상태

파일 이력 창이 기존에 보유한 `shortcut::Bindings` 복사본을 Diff 포커스에서도 사용한다. 새로운 저장 상태는 없다. 편집기의 전체 텍스트 버퍼와 커서·선택 상태는 기존 가상 스크롤 구조를 유지한다.
## US162 빈 그리드 배경

커밋 항목과 목록 길이는 변하지 않는다. 목록 패널의 빈 영역에는 헤더/행의 네 열 폭과 같은 화면용 가이드 선만 표시한다. 새로운 저장 상태는 없다.
## US163 Diff 빈 영역

새 Diff 줄이나 텍스트 버퍼 내용은 추가하지 않는다. 캔버스의 화면 높이에만 번호 열 배경과 번호·기호 경계선을 연장해 그린다.
## US164 기존 DiffViewer 열 표시

Diff 데이터와 선택 상태는 바꾸지 않는다. 실제 줄의 번호·기호 셀 및 빈 영역 배경 가이드를 그리는 화면 표현만 추가한다. 부분 Stage의 체크 셀과 가로 스크롤 오프셋은 기존 상태를 사용한다.
## US165 Diff 선택 상태

기존 `text_selection`과 `text_drag` 상태를 유지한다. 커서가 허용되는 행 범위는 실제 Diff의 모든 줄로 넓어지며, 부분 Stage 체크 상태는 추가·삭제 줄만 대상으로 유지한다. 뷰어 밖 드래그는 뷰포트와 스크롤 오프셋으로 계산한 실제 행·열을 선택 끝점에 기록한다.

## US184 파일 이력 열 너비 화면 상태

파일 히스토리 창은 Author·Date·Hash 열의 너비와 현재 드래그 경계·시작 포인터·시작 너비를 화면 상태로 가진다. Description은 창의 남은 너비를 사용한다. 창을 닫으면 이 상태는 사라지며 검색 결과·커밋 데이터나 저장 파일 형식은 바뀌지 않는다.

## US185 신규 저장소 디렉터리

선택한 기존 디렉터리가 작업 트리 루트가 된다. 그 안의 `.git` 디렉터리가 저장소 메타데이터를 보유한다. Windows에서 `.git`의 숨김 속성만 추가하며 기존 작업 트리 파일과 사용자 설정 저장 형식은 변경하지 않는다.

## US187 Clone 대상 디렉터리

사용자가 선택한 기존 빈 디렉터리가 복제 결과의 작업 트리 루트가 된다. 원격 설정과 Git 메타데이터는 그 안의 `.git`에 저장된다. 새 `repository` 하위 폴더나 앱 설정 형식은 추가하지 않는다.

## US188~191 새 탭 화면 상태

영구 삭제는 기존 저장소 루트 검증 결과 경로에 적용되며 새 영속 데이터는 없다. 새 탭 버튼의 포커스 핸들 네 개와 최근 저장소 경로별 열기·삭제 포커스 핸들은 창 메모리에만 보관한다. Clone URL 입력창은 기존 편집기를 사용한다. 새 탭 단축키는 기존 단축키 설정 형식의 `new-tab` 명령으로 저장된다.

## US192 탭 닫기 명령

단축키 설정의 기존 매핑 형식에 `close-tab` 명령을 추가한다. 탭 삭제 후 세션 상태는 기존 `close_tab`의 저장 형식을 사용하며 저장소 파일과 Git 데이터는 변경하지 않는다.
