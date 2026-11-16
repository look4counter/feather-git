# 구현 계획: 저장소 항목 삭제

## 기존 구조 분석
Rust/GPUI/git2. ui.rs의 open_checkout_menu는 로컬·원격·태그와 커밋 행에서 사용되며 스태시는 체크아웃 대상에서 제외되어 메뉴가 열리지 않는다. repository::panel_references는 스태시 인덱스와 설명을 제공한다. ToolbarAction/Request/Result는 백그라운드 Git 작업·전역 진행·오류·snapshot 갱신을 공유한다. committing::push_to는 인증과 서버 거부를 처리하지만 삭제의 예상 객체 검증이 없으므로 삭제용 콜백이 필요하다.

## 설계
reference_actions.rs에 삭제 준비와 실행을 모으고 repository.rs 및 backend.rs 경계로 노출한다. 준비 결과는 정확한 참조·객체 ID·원격 매핑이다. 원격은 fetch refspec을 역변환해 유일한 서버 refs/heads 대상을 찾는다. push negotiation에서 서버 기존 ID를 비교하고 삭제 refspec을 전송한다. 서버 거부와 연결 오류는 비밀 주소 없이 표시한다. 성공 후 로컬 참조를 정리하며 부분 실패를 구분한다.

로컬은 HEAD와 참조 잠금 후 현재·다른 worktree 사용 여부 및 HEAD 또는 upstream 병합 여부를 확인한다. 태그는 확인 객체와 비교 후 참조 transaction으로 삭제한다. 스태시는 refs/stash를 잠그고 reflog의 인덱스·객체를 비교한 뒤 해당 항목만 제거하고 HEAD와 reflog를 함께 갱신한다. 파일·인덱스는 건드리지 않는다.

UI는 참조 메뉴 대상 검증을 공통화하고 네 탭 모두 삭제 메뉴를 표시한다. 체크아웃은 기존 세 참조 탭과 커밋에서 유지한다. 삭제 준비는 비동기로 수행하고 세대·프로젝트·경로·탭·선택을 재검사한다. 별도 삭제 확인 상태에 준비 결과와 소유자를 저장하고 확인·취소·Escape·Tab·포커스 복원을 제공한다. 확인 후 기존 ToolbarAction의 삭제 변형으로 실행한다. 작업 완료 재조회로 네 탭·기록을 갱신한다.

## 제약·검증·헌법
새 의존성 없음. 기존 공개 계약 호환성과 계층을 유지한다. 헌법은 미작성 템플릿이므로 규범 검사는 생략한다. AGENTS.md의 최소 변경·명시 확인·기존 변경 보존을 따른다. 임시 저장소와 로컬 bare 원격에서 네 종류 삭제·변경 대상·현재 브랜치·미병합·스태시 순서 변경·서버 변경·HEAD/파일 보존을 검증한다. UI 순수 상태 검사는 메뉴 대상·취소·오래된 요청을 검증한다. fmt·전체 test·check·엄격한 clippy·별도 실행 파일 빌드를 수행한다. 실제 외부 서버 인증과 GUI는 미검증 시 보고한다.

## 기존 로컬 브랜치 재사용 추가 설계
ui.rs의 start_toolbar_action에서 remote_branch_label로 기본 이름을 구하고 Project.branches에 존재하면 기존 refs/heads 체크아웃으로 라우팅한다. 없으면 New Branch에 정확한 기본 이름을 전달한다. branch_dialog.rs의 configure_remote는 기본 이름 인수를 받는다. branch_actions.rs의 create_tracking_branch는 실행 시에도 동명 로컬을 확인하여 기존 브랜치를 안전하게 체크아웃하고 upstream은 변경하지 않는다. 일반 create_branch의 중복 거부는 유지한다. 이전 기능 명세의 원격 중복 거부는 새 사용자 지시로 이 동작에 한해 대체한다. 격리 테스트로 다른 커밋·다른 upstream을 가진 동명 로컬의 보존과 충돌 보호를 검증한다.

삭제 확인 화면에는 정확한 참조·객체 ID와 스태시의 실제 reflog 설명을 표시한다. 원격 처리 중 전송 또는 로컬 갱신 오류는 서버 반영 여부 확인이 필요함을 안내한다. 서버 성공 후 로컬 정리에서 발생한 오류는 서버 삭제 완료 사실을 명시한다.
