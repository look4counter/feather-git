# 내부 계약

synchronize는 사용자 선택된 모드만 실행하고 오류를 Result로 전달한다. 충돌 오류도 실제 저장소 상태를 보존한다. workspace 요청은 성공 여부와 관계없이 새 스냅샷을 읽으며 UI는 이를 따로 반영한다. UI 스레드에서 Git I/O를 수행하지 않는다. 네트워크 오류에 URL 비밀값을 노출하지 않는다.

SplitButton은 실행 콜백과 선택 콜백을 구분한다. 선택은 저장 상태만 변경하며 본체만 synchronize 요청을 시작한다. 선택 상태는 Session의 기본값 있는 pull_mode 필드에 안정 키로 저장한다. busy/disabled는 본체·화살표를 모두 차단한다.

UI의 선택 가능 옵션은 Fetch·일반 Pull·빠른 전진 전용 Pull 세 개다. 저장된 rebase 선택은 일반 Pull로 복원하고 UI에서 rebase 모드를 선택·실행하지 않는다. 내부 리베이스 동작은 유지한다.
