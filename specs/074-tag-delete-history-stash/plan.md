# 구현 계획

## 기존 구조와 영향 분석
Rust/GPUI/git2. reference_actions.rs는 객체 ID를 비교하고 remote push negotiation으로 서버 삭제를 보호한다. 태그 생성의 원격 선택은 tag_actions.rs 안에 있어 공용 순수 정책으로 추출한다. ui.rs 태그 삭제 팝업은 확인·취소 포커스를 보관한다. 커밋 기록은 virtual_list로 클릭 횟수를 받지 않지만 기존 multi_virtual_list가 MouseDownEvent를 제공하므로 이를 재사용한다. BranchDialog는 브랜치·태그 이름/체크박스·IME·busy·오류 흐름을 공유하며 Stash 모드를 추가할 수 있다. ToolbarAction/Request/Result가 비동기 실행과 snapshot 갱신을 처리한다.

## 설계
태그 삭제 상태에 원격 옵션을 추가하며 새 팝업마다 false로 초기화한다. 태그 팝업에만 체크박스와 포커스를 제공한다. 기존 delete_reference API는 유지하고 원격 태그 옵션용 경로를 추가한다. 기존 요청 객체 검증 후 태그에 한해 기본 Push 원격과 refs/tags 대상 매핑을 사용하고 기존 서버 삭제·로컬 정리를 공유한다. 태그 서버 미존재는 성공으로 처리하되 다른 객체를 가리키면 거부한다. 원격 선택은 tag_actions의 기존 정책을 공용 함수로 추출해 생성과 삭제가 공유한다.

커밋 기록은 multi_virtual_list로 전환하고 왼쪽 정확히 두 번 클릭을 객체 ID 체크아웃 요청으로 연결한다. 기존 start_toolbar_action과 detached 확인을 사용한다. 단일클릭·방향키는 select_history_row만 사용한다. 프로젝트·행·로딩·작업 상태를 검사한다.

stash_actions.rs는 필수 이름·작업 상태·HEAD·서명·인덱스 충돌을 검사하고 git2::stash_save와 선택적 KEEP_INDEX를 사용한다. 미추적 파일 포함 플래그는 사용하지 않는다. ToolbarAction::CreateStash와 backend/repository API로 노출한다. BranchDialog에 Stash 모드를 추가하고 기존 팝업 소유자·취소·포커스·busy 처리를 재사용한다. split_button.rs에 기존 선 아이콘 스타일로 stash 아이콘을 만들고 Branch 바로 오른쪽에 버튼을 둔다. 완료는 기존 snapshot과 패널 갱신을 사용한다.

## 제약·검증·헌법
새 의존성·공개 계약 파괴 없음. 헌법은 미작성 템플릿으로 유효 규범 검사 생략. AGENTS.md의 한글 문서·기존 변경 보존·재사용·최소 변경을 준수한다. 임시 저장소와 로컬 bare 원격에서 태그 삭제 옵션·대상 보호·keep-index 양쪽·추적/미추적 파일·스태시 이름·오류를 검증한다. UI 상태 테스트로 클릭 횟수·로딩·오래된 대상·팝업 옵션을 검증한다. fmt·전체 test·check·엄격한 clippy·별도 빌드 후 수렴 점검한다. 실제 GUI와 외부 서버 인증은 미검증으로 보고한다.

## 구현 검증에서 확인한 사항
Stash 테스트의 기존 Repository 핸들은 별도 backend 핸들이 갱신한 인덱스를 캐시하고 있었다. 검증 시 index.read(true)로 실제 파일을 다시 읽도록 수정했으며 staged·unstaged 내용과 keep-index 양쪽의 동일한 기대 조건을 유지했다.
커밋 더블클릭은 공용 리스트의 포커스 처리 후 확인 팝업을 열도록 defer하며, 실행 직전에 같은 프로젝트·행·ID를 다시 검사한다.
