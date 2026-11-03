# 검증 절차

## 자동 검증

저장소 루트에서 `cargo test --offline --bin feather-git commit_editor::tests`를 실행한다.
메시지 순서 회귀 검증과 기존 편집기 검증이 통과해야 한다.
`cargo build --offline`로 실제 Windows 메시지 연결까지 컴파일되는지 확인한다.

## 수동 입력 확인

빌드한 애플리케이션에서 커밋창을 열고 Microsoft 한국어 입력기로 `쿼리` 입력 후 한/영 전환하여 `ID`를 입력한다. 결과는 `쿼리ID`여야 한다. 왼쪽 이동·삭제·재입력도 가능해야 한다. 이모지와 여러 줄 접두사, 한글 연속 조합, 조합 취소도 확인한다.

수동 확인을 실행하지 못한 경우 자동 검증과 구분하여 결과에 명시한다.

## 실행 결과 — 2026-11-03

- 수정 전 상태 전이를 연결한 실제 버퍼 검증에서 4개 중 3개가 실패했다. `쿼I`로 덮어쓰기, 새 조합 손실, 종료 후 조합 표시 잔존을 확인했다.
- 수정 후 새 회귀 검증 4개가 통과했다.
- `cargo test --offline --bin feather-git commit_editor::tests`: 기존 7개와 새 4개, 총 11개 통과.
- `cargo build --offline --bin feather-git`: 성공. 결과는 `target/debug/feather-git.exe`.
- 변경 소스의 형식 검사와 변경된 기존 파일의 공백 검사가 통과했다.
- 원본 GPUI와 파일 해시 비교 결과 의도한 Windows 모듈 4개 및 한글 설명 문서만 변경되었다. 원본 라이선스는 동일하다.
- GPUI의 기존 부동소수점 추론 경고와 의존성의 향후 호환성 경고가 있으나 빌드를 막지 않는다.
- 실제 Microsoft 한국어 입력기로 수행하는 수동 입력 확인은 실행하지 않았다. 원래 보고된 종료 현상의 스택이 없어 별도 종료 원인까지 해결했다고 단정하지 않는다.

## 재조사 결과 — 완료 아님

사용자는 `cargo run`으로 debug 실행 파일에서 증상을 재현했다. 오래된 release 파일은 이번 재현의 원인이 아니다. release 파일도 로컬 GPUI 패치를 포함하여 다시 빌드했지만 이것을 문제 해결 증거로 사용하지 않는다.

Computer Use로 Feather Git을 실행하려 했으나 `Computer Use was not approved to use feather-git`로 거절되었다. UI 입력을 다른 수단으로 우회하지 않았다.

진단 실행은 저장소 루트의 PowerShell에서 아래와 같이 수행한다.

```powershell
$env:FEATHER_IME_TRACE = 'D:\Projects\feather-git\.build\ime-trace.log'
cargo run
```

커밋창에서 `쿼리` → 한/영 전환 → `I`를 입력한 뒤 `.build/ime-trace.log`를 확인한다. 진단은 본문 없이 메시지 종류·조합 상태·교체 길이만 기록한다. 진단 비활성화는 해당 셸에서 `Remove-Item Env:FEATHER_IME_TRACE`로 수행한다.

진단 추가 후 기존 편집기 검증 11개는 통과했다. 실제 이벤트 로그 확보, 그에 따른 수정, 실제 입력 재확인은 아직 미완료다.

## 실제 로그 기반 수정 검증

- 재현 로그에서 조합 영역 `1..2`가 남은 상태의 `WM_CHAR`가 `replace bytes=1`로 한글을 교체하고, 이후 `GCS_RESULTSTR`가 도착하는 순서를 확인했다.
- 네이티브 조합이 있으면 일반 문자를 삽입하기 전에 Windows 입력기에 CPS_COMPLETE 확정을 요청하도록 변경했다. 재진입 호출 동안 창 상태나 입력 핸들러를 빌리지 않는다.
- 관측된 순서의 회귀 검증은 수정 전 `쿼I`로 실패했고 수정 후 통과했다.
- `cargo test --offline --bin feather-git commit_editor::tests`: 13개 통과.
- 네이티브 조합 없는 dead-key 입력 회귀 검증도 통과했다.
- 형식 검사와 변경된 기존 파일 공백 검사 통과.
- 수정 후 실제 사용자 입력 재확인 대기 중. 완료로 판정하지 않았다.

## 한/영 전환 키 검증

- 공통 메시지 루프에 TranslateMessage 호출이 없으며, 키 변환에 실패한 기존 경로에도 호출이 누락되어 있음을 확인했다.
- 입력기 처리 키와 입력기 제어 키를 일반 키 변환 전에 Windows에 전달하도록 수정했다.
- 전환 키 전달 회귀 검증은 수정 전 실패, 수정 후 통과했다. 조합 중 및 조합하지 않는 상태에서 양방향 전달을 다룬다.
- 오른쪽 Alt·일반 Alt·일반 문자·기존 VK_PACKET 경로를 변경하지 않는 검증도 통과했다.
- `cargo test --offline --bin feather-git commit_editor::tests`: 15개 통과.
- 최신 사용자 재현 로그에서 앞선 글자 보존 수정의 `complete_native success=true` 뒤에 `marked_before=None`으로 영문이 삽입된 것을 확인했다. 본문을 기록하지 않으므로 최종 문자열의 시각적 확인은 별개다.
- 한/영 전환 수정 후 실제 왕복 전환과 소문자 입력 확인은 대기 중이다.

## 최종 실제 입력 확인 — 2026-11-03

사용자가 수정 후 같은 진단 터미널에서 `cargo run`으로 실행하여 빈 입력창의 한/영 왕복 전환과 한글 조합 중 전환 후 소문자 입력을 확인하고 “잘되네”라고 응답했다.

최신 로그에서도 한/영키의 VK_PROCESSKEY가 `native_ime_key translated=0xe5`로 전달되고, 조합 중 전환에서 확정 결과와 종료 알림이 처리되는 것을 확인했다. 이전의 입력기 키 번역 누락 경로가 제거되었다.

자동 검증 15개와 실제 사용자 확인으로 FR-001~006 및 SC-001~004의 해당 입력 범위를 검증했다. 처음 보고된 별도 비정상 종료의 원인은 스택을 확보하지 못했으므로 모든 종류의 종료 현상을 해결했다고 확대하지 않는다.

진단이 끝났으므로 앱을 종료한 뒤 해당 PowerShell에서 `Remove-Item Env:FEATHER_IME_TRACE`를 실행하면 다음 실행부터 로그를 남기지 않는다.

- 최종 `cargo build --offline --bin feather-git` 및 `cargo build --offline --release --bin feather-git` 모두 성공했다. debug와 release 실행 파일에 동일 수정이 포함된다.
