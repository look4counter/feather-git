# 구현 계획

## 기존 분석
Rust/GPUI. virtual_list.rs가 메인 저장소·파일·커밋과 팝업 목록을 공유한다. 선택 콜백은 조회 등 기존 동작을 수행한다. diff_viewer.rs의 가상 목록은 텍스트 선택에 쓰이므로 키보드 선택을 제외해야 한다. ui.rs는 panel_focus로 파일 단축키를 제한한다. 최근 저장소는 별도 스크롤 목록이고 설정 원격·인증 항목은 form_controls::list_row와 삭제 버튼용 포커스 맵을 사용한다.

## 설계
공용 가상 목록에 수정키 없는 방향키 판별·경계 이동 함수를 추가하고 기존 선택 콜백을 재사용한다. 목록 포커스는 보존되는 스크롤 핸들에 유지하고 마우스·키보드 선택 후 보존한다. 선택 행 자동 스크롤 및 비활성·로딩 차단을 적용한다. diff 목록은 명시적으로 제외한다. 파일 단축키의 부모 포커스 검사를 자식 포함 검사로 바꾼다. 최근 목록은 선택 상태 및 포커스·스크롤을 추가하며 Enter만 열기를 수행한다. 설정 목록은 기존 삭제 포커스와 별도의 행 포커스로 이동하고 시각적으로 표시한다.

체크아웃은 start_toolbar_action의 공통 진입점에서 원격은 New Branch 팝업으로 연결하고, 태그와 커밋 ID 대상은 경고 확인을 위해 보류한다. 보류 상태에 프로젝트·경로·대상·포커스를 보관하고 확인·취소 대화상자를 표시한다. 확인 시 보류 상태를 먼저 제거하고 프로젝트·경로·작업 가능 상태를 재검사한 뒤 내부 실행 함수로 진행한다. Enter 확인과 Escape 취소, 버튼 Tab 이동을 지원한다. 취소는 Git 호출 없이 포커스를 복원한다. 기존 백엔드 안전성은 유지한다.

## 영향 및 검증
변경 파일: virtual_list.rs, diff_viewer.rs, ui.rs, settings.rs, auth_settings.rs, form_controls.rs, branch_dialog.rs, repository.rs, branch_actions.rs, backend.rs, workspace_backend.rs. 새로운 의존성 및 공개 계약 파괴 없음. 헌법은 미작성 템플릿으로 규범 검사 생략. AGENTS.md의 계층·재사용·최소 변경 준수. 경계 이동·수정키·선택 동기화·detached 판별과 보류 취소·오래된 요청을 단위 테스트로 검증한다. fmt, 전체 test, check, 엄격한 clippy 실행. 실제 GUI는 미검증 시 명시한다.

## 추가 배지 요구 분석 및 설계
repository.rs의 snapshot은 로컬·원격 이름만 읽으며 추적 관계는 Project에 없다. Project에 로컬 이름 → 원격 upstream 표시 이름 맵을 추가하고 snapshot의 백그라운드 Git 조회에서 실제 upstream을 해석한다. upstream 없음은 빈 관계로 처리하고 다른 조회 오류는 숨기지 않는다. 로컬 upstream은 원격 배지에서 제외한다. ui.rs 로컬 행에서 현재 배지를 맨 앞에 두고 브랜치명 뒤에 원격 배지를 표시한다. 모든 기존 snapshot 갱신에서 관계를 함께 갱신한다. 임시 저장소에서 다른 이름의 upstream·없음·로컬 upstream을 검증한다.

## 원격 체크아웃 변경 설계
추가 사용자 지시로 원격은 detached 경고 대상에서 빠지고 New Branch 팝업으로 연결한다. BranchDialog에 원격 생성 모드를 추가하여 대상·기본 이름을 표시하고 체크아웃을 고정한다. UI는 원격 대상과 기존 branch_owner를 보존한다. ToolbarAction에 추적 브랜치 생성 변형을 추가하고 branch_actions.rs의 원격 커밋에서 생성·upstream 설정·기존 안전한 로컬 checkout 순서를 사용한다. backend.rs/repository.rs 경계로 노출한다. 실패 시 이미 생성된 브랜치가 있으면 사실을 포함한 오류를 반환한다. 중복 이름 덮어쓰기 없음. 기존 일반 New Branch는 그대로 유지한다.

## 추적 배지 표시 변경 분석 및 계획
현재 로컬 upstream은 전체 이름을 회색 텍스트 배지로 표시한다. 커밋 기록은 repository.rs의 원격 이름 접두사 제거 후 ui.rs의 원격 아이콘·0xffa7b2 글씨 배지로 렌더링한다. 표시 변경은 Git 작업이나 선택 경로에 영향을 주지 않는다.

기존 접두사 제거 로직을 순수 함수 remote_branch_label로 추출하고 커밋 기록과 로컬 탭에서 공유한다. Project에 설정된 원격 이름 목록을 보존해 slash가 포함된 원격도 가장 긴 실제 접두사로 처리한다. backend.rs는 순수 함수를 재노출한다. ui.rs는 커밋 기록의 배지 구성을 reference_badge로 추출해 재사용한다. 로컬명 뒤에 화살표와 같은 원격 배지를 추가한다. 전체 upstream 식별자는 유지한다. 새 의존성·I/O·체크아웃 변경 없음.

검증: 기존 커밋 참조 이름 테스트와 upstream 테스트, 새 이름 축약 경계 테스트, fmt·check·엄격한 clippy와 별도 이름 실행 파일 빌드를 수행한다. 이번 변경은 표시 중심이므로 전체 회귀 반복은 새로운 실패가 없는 한 생략한다. 실제 GUI 확인은 미검증으로 명시한다. 헌법은 미작성 템플릿이며 AGENTS.md의 최소 변경·기존 구현 재사용을 유지한다.
