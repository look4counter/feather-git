# 구현 계획: 안전한 강제 Push

## 기술 환경과 기존 흐름

Rust 2024·GPUI 0.2.2·git2 0.21.0이다. `ui.rs`가 GitActionDialog 옵션을 확인한 후 ToolbarAction을 백엔드로 넘긴다. `branch_actions::push_branch`는 업스트림 또는 origin과 현재 브랜치를 결정하고 `committing::push_to`로 인증·전송·서버 거부를 처리한다. 일반 Push의 전송 생략 판단은 로컬이 뒤에 있는 경우도 생략하므로 안전한 강제 Push에는 적용하지 않는다.

## 설계와 영향 범위

- `src/git_action_dialog.rs`: ActionOptions::Push(bool)와 기존 체크박스·Space 처리 재사용. 기본 false.
- `src/ui.rs`: Push 옵션을 확인 후 Push 또는 PushWithLease 작업으로 변환한다. 기존 대상 변경·취소 검사를 유지한다.
- `src/workspace_backend.rs`: PushWithLease를 기존 백그라운드 Push 경로로 전달한다.
- `src/branch_actions.rs`, `src/repository.rs`: 기존 push_branch API를 유지하며 push_branch_with_lease를 추가한다. 공통 내부 함수에서 대상 선택·최초 업스트림 설정을 공유한다.
- `src/committing.rs`: 기존 push_to를 유지하고 선택적 lease OID를 받는 내부 전송 함수를 추가한다. 대상 원격의 fetch refspec으로 로컬 추적 참조를 찾아 네트워크 연결 전에 기준값을 읽는다. 참조 부재는 zero OID, 그 외 오류는 실패다. push_negotiation에서 update.src()인 서버 현재 값과 lease를 비교하고 일치할 때만 +refspec을 허용한다. 서버가 협상 이후 변경되는 경우는 Git 수신 측의 기존 old-OID 검사를 사용한다. 자동 fetch로 기준을 갱신하지 않는다.

## 제약 및 헌법 점검

새 의존성, 공개 API 제거, 저장 형식 변경은 없다. 기존 인증 콜백과 태그 덮어쓰기 방지를 보존한다. 헌법은 미작성 템플릿이므로 구체 검사 생략, 기존 계층과 최소 변경 규칙은 준수한다. 구현 전후 동일하게 점검한다.

## 검증

로컬 bare 원격 통합 테스트로 재작성 성공, 추적 값 불일치 거부, 기준 없는 원격 존재 거부·부재 생성, 사용자 지정 대상과 원격을 검증한다. 팝업 Space·확인 옵션 전달·취소·stale 검사를 테스트한다. 기존 Push·태그·커밋 후 Push 테스트와 cargo check·Clippy를 실행한다. 실제 외부 원격에 푸시하지 않는다. GUI 직접 조작 여부를 별도 기록한다.

구현 중 확인: repository 공개 함수는 `src/backend.rs`의 래퍼 매크로를 통해 노출된다. 기존 계층과 계측을 유지하도록 새 함수를 동일 매크로에 등록한다. T004의 영향 파일에 `src/backend.rs`를 포함하며 추가 일관성 분석 결과 FR-003의 기존 백엔드 경로 사용과 일치하고 충돌은 없다.
