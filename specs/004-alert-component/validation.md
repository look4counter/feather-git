# 검증 결과: 재사용 alert 팝업

## 구현과 명세 일치

- src/alert.rs의 Alert::new는 제목/메시지를 받고 확인 문구를 제공한다. render는 포커스와 닫기 콜백을 받는다. 컴포넌트에서 Workspace나 저장소에 의존하지 않는다.
- 중앙 모달, 어두운 전체 배경, 최대 가용 너비/높이와 스크롤 카드, 확인 버튼을 구현했다. occlude 및 마우스 이벤트 차단으로 배경 입력을 막고 배경 클릭으로 닫지 않는다.
- 확인·Enter·Esc는 같은 콜백을 사용한다. Create에서 이전 포커스를 보관하고 팝업에 포커스를 주며 닫기 시 이전 포커스를 복원한다.
- Create는 테스트 알림을 열고 Open/Clone은 기존 안내를 유지한다. alert는 Option 하나이므로 중복으로 쌓이지 않으며 새 Workspace에는 없다.
- 기존 색상 값을 theme.rs로 이동해 UI와 alert가 공유한다. 의존성이나 저장소 접근/영속화는 추가하지 않았다.
- FR-001~005의 코드 대응을 확인했다. 실제 입력·표시 검증 한계는 아래와 같다.

## 자동화 검증

- cargo fmt --check: 통과.
- cargo check --offline: 통과.
- cargo test --offline: 21개 통과. 새로 추가한 alert 내용 보존, 닫기 키 판별, Create 상태/중복/닫기/재열기/비영속 검증 포함.
- 테스트 매크로 충돌은 Rust 기본 테스트 경로를 명시해 해결했다. 테스트 조건 완화는 없다.
- cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-alert.exe: 통과. 테스트 매크로 수정은 테스트 빌드에만 적용된다.
- 기존 proc-macro-error2 미래 Rust 호환성 경고와 별도 출력 경로 안내가 있다.

## 미검증 항목

이전 화면 도구의 실행 파일 사용 승인 제한으로 실제 Create 클릭, 중앙 배치/긴 메시지 줄바꿈, 배경 클릭 차단, 확인·Enter·Esc 입력과 포커스 복원은 직접 확인하지 못했다. 자동화 테스트가 실제 시각·포커스 검증을 대신한다고 보고하지 않는다.
