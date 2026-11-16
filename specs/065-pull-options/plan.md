# 구현 계획

## 기술 및 기존 구조 분석
Rust/GPUI와 git2 0.21을 사용한다. UI는 backend::spawn을 통해 Git 작업을 실행하고 공용 context_menu를 드롭다운으로 재사용한다. repository.rs의 인증 콜백과 open_exact을 재사용하는 synchronization.rs 하위 모듈을 추가한다. SyncMode 네 값과 synchronize를 backend 경계로 노출한다. 명령은 동작 명세이며 OS Git 프로세스를 새 실행 경로로 도입하지 않는다.

## Git 처리 흐름
Fetch는 현재 브랜치의 remote 또는 origin에서 설정 refspec을 가져온다. Pull은 HEAD가 로컬 브랜치이고 upstream이 구성되어 있는지 확인한 뒤 fetch하고 upstream의 새 커밋을 읽는다. 동일/로컬만 앞섬은 종료한다. 원격이 앞서면 안전한 checkout 및 참조 잠금으로 fast-forward한다. 분기된 경우 FF-only는 오류, 기본 모드는 git2 merge 후 충돌이 없으면 두 부모 병합 커밋, rebase는 git2 rebase 반복과 원래 author/message를 사용해 커밋하고 finish한다. 충돌은 중간 상태를 보존하고 안내하며 강제 reset·자동 stash는 하지 않는다. 현재 진행 중인 Git 통합 작업은 재시작하지 않는다. 쓰기 전 시그니처와 참조 잠금 오류를 확인하고 checkout safe를 사용한다.

## UI 및 갱신
툴바의 샘플 버튼 7개를 Pull 메뉴 하나로 교체한다. sync_busy를 공통 작업 가드에 포함하며 실행 중에는 기존 전역 ProgressOverlay로 상태를 표시한다. 메뉴 선택은 옵션만 저장하고 본체 클릭에서만 실행한다. 작업 결과와 별도로 저장소 스냅샷 및 PreparedHistory를 백그라운드에서 만든다. 성공/실패 모두 현재 저장소와 일치하는 결과만 반영하고 기록 커서·브랜치·파일 상태를 함께 교체한다. 기존 선택은 가능한 범위에서 유지하고 작업 오류는 공용 알림으로 표시한다.

## 영향 및 불변 조건
repository.rs, synchronization.rs, backend.rs, workspace_backend.rs, ui.rs를 변경한다. 헌법은 미작성 템플릿이다. UI/비즈니스/저장 접근 책임과 인증 프로필 호환성을 유지한다. 새 의존성과 전역 설정 변경은 없다. 테스트는 임시 로컬 원격 저장소만 사용하며 실제 네트워크/계정은 조작하지 않는다.

## 검증
임시 원격/로컬 fixture로 Fetch의 HEAD/파일 보존, 세 Pull 모드의 fast-forward, 분기 merge 부모 및 rebase ancestry, FF-only 거부, 최신·로컬 앞섬, 충돌, upstream/HEAD/원격 없음, 로컬 변경 및 잠금 보호를 테스트한다. 작업 가드와 화면 갱신을 검증하고 전체 test·fmt·엄격한 clippy·빌드를 실행한다. 실제 외부 인증/GUI 미검증은 명시한다.

## 잠금 검증 보강

libgit2 checkout이 기존 인덱스 잠금을 뒤늦게 확인하는 경우를 회귀 테스트에서 확인했다. 작업 파일 갱신 전에 index.lock을 배타적으로 확보하고 같은 디렉터리의 잠금 인덱스를 Repository::set_index로 연결한다. 성공 및 충돌 중간 상태는 원본 인덱스에 반영하고 일반 실패는 원본을 보존한다. 기존 참조 잠금과 checkout safe를 함께 사용한다. 테스트의 줄바꿈은 임시 .gitattributes로 고정하여 사용자의 autocrlf 전역 설정에 영향을 받지 않는다.

Pull에서는 FETCH_HEAD의 실제 가져온 참조를 현재 branch.merge와 비교한다. 원격에서 upstream이 삭제되었어도 남아 있는 추적 참조를 최신으로 오인하지 않도록 검증한다. Fetch 자체는 기존 Git 기본값처럼 자동 prune하지 않는다.


## 공통 SplitButton 변경 계획

src/split_button.rs에 공통 RenderOnce 컴포넌트를 추가한다. 아이콘/하단 문구와 실행 콜백, 옵션 목록/선택 인덱스와 선택 콜백, 비활성·작업 중 상태를 받는다. 본체와 화살표 이벤트를 분리하고 기존 context_menu를 사용한다. context_menu의 선택 체크 표시를 선택적으로 추가하며 일반 메뉴의 기존 모양은 유지한다. 클릭 위치 대신 컴포넌트 아래에 메뉴를 고정한다.

Workspace는 selected_sync_mode를 갖고 메뉴 콜백은 이 값만 갱신·저장한다. 본체 콜백만 start_sync를 실행한다. Session에 기본값이 있는 pull_mode 문자열 필드를 추가하고 SyncMode의 안정 키 변환으로 복원한다. 기존 version 1 파일은 serde(default)로 호환되며 알 수 없는 키는 Merge로 복원한다. 툴바는 높이60/가로 여백12로 줄이고 오른쪽 문구·배지를 제거한다. 아이콘은 기존 canvas 체계의 아래쪽 화살표와 받침 모양을 사용한다.

영향 파일은 split_button.rs, main.rs, ui.rs, context_menu.rs, session.rs, synchronization.rs이다. Git 실행 계층은 변경하지 않는다. 선택 키 왕복·이전 JSON 기본값·선택 저장과 재시작 복원·선택만으로 실행하지 않음·작업 중 변경 거부를 검증하고 전체 회귀 및 빌드를 실행한다.

## Hover 및 라벨 변경 계획

src/split_button.rs의 기본 RAISED 배경만 제거한다. 본체와 화살표의 기존 활성 조건부 hover LINE 배경 및 구분선을 유지한다. src/ui.rs의 toolbar 생성 시 선택 모드를 기준으로 Fetch 또는 Pull 문구를 전달한다. 선택 저장과 Git 실행 계층은 변경하지 않는다. 단순 표시 변경이므로 구현을 복제하는 테스트는 추가하지 않고 기존 옵션 유지·실행 가드 테스트와 fmt·엄격한 clippy·별도 빌드 및 코드 대조로 검증한다.

## 전역 진행 표시 변경 계획

src/ui.rs의 기존 ProgressOverlay 표시 조건에 sync_busy를 포함하는 공통 판정을 사용한다. start_sync에서 기존 progress_focus/progress_previous_focus를 설정하고 완료 콜백은 요청 세대가 같은 경우에만 해제·복원한다. 오류 알림 전에 포커스를 복원한다. 기존 작업 가드가 저장소 열기/삭제와 원격 작업의 동시 실행을 막으므로 진행 상태를 재사용할 수 있다. start_sync/finish_sync의 notice 쓰기는 제거한다. 기존 성공·실패·오래된 결과 테스트에 오버레이 표시 상태와 하단 문구 보존 검증을 추가하고 관련 테스트·fmt·엄격한 clippy·빌드를 실행한다.

## 작은 진행 카드와 타이틀바 제외 계획

progress.rs의 ProgressOverlay에 top_inset 속성을 추가한다. 루트는 top/left/right/bottom으로 작업 영역만 덮고 반투명 검정 배경막과 기존 이벤트 차단을 유지한다. 기존 깃털 애니메이션과 문구를 너비280의 PANEL 카드에 넣는다. ui.rs의 Windows 타이틀바 높이32를 상수로 공유하고 전역 오버레이 호출에만 해당 inset을 전달한다. 비 Windows는 네이티브 타이틀바가 클라이언트 밖에 있으므로 0을 전달한다. 다른 팝업이나 시작 화면은 변경하지 않는다. 기존 프로그레스/작업 상태 테스트, fmt·엄격한 clippy·별도 빌드와 코드 대조로 검증한다. 실제 화면 배치는 미검증으로 보고한다.

## Pull rebase 옵션 제거 계획

기존 호출 분석: synchronization.rs의 SyncMode::ALL이 메뉴·선택 인덱스·키 왕복 테스트를 공유한다. ui.rs는 from_key로 저장값을 복원하고 choose_sync_mode와 start_sync에서 실행한다. SyncMode::Rebase와 내부 rebase는 기존 Git 계층·회귀에서 쓰며 기록 리베이스가 재사용할 수 있다.
설계: 세 가지 UI 옵션 목록 OPTIONS를 정의하고 메뉴의 단일 출처로 사용한다. 기존 키 파서는 유지하되 UI용 from_option_key로 rebase 저장값을 Merge로 정규화한다. 선택과 실행에서도 OPTIONS를 검증한다. 백엔드 리베이스·외부 작업 상태 표시·세션 형식은 보존한다. 수정은 src/synchronization.rs·src/ui.rs와 기존 테스트에 한정한다. 헌장은 미작성 템플릿이며 기존 구조 원칙과 충돌 없음.
검증: 정확한 세 옵션과 순서·이전 rebase 저장값 복원·제거 모드 선택 거절·세 옵션 저장·실행 보호 및 기존 Git 회귀·컴파일·정적 분석·별도 빌드를 확인한다.
