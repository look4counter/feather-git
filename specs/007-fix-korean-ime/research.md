# 조사 기록

## 결정

GPUI 0.2.2를 로컬 패치로 고정하고 Windows 메시지 경로를 수정한다.

## 근거

- 로컬 `events.rs`는 조합 종료 메시지를 처리하지 않고 일반 문자를 현재 조합 영역의 교체로 전달한다.
- `Buffer::replace`는 남아 있는 조합 영역을 우선 교체하므로 `쿼리` 뒤 `I`가 `리`를 덮어쓴다.
- 동일 메시지에서 새 조합을 먼저 적용하는 순서도 이전 확정 결과가 새 조합을 덮는 원인이다.
- 종료 뒤 확정 결과가 늦게 전달되는 순서를 허용해야 한다. 종료 표시를 다음 독립 입력 전까지 보류하고 확정 결과가 도착하면 기존 조합을 교체한다.
- 별도 연구 에이전트의 읽기 전용 조사로 메시지 처리 누락과 한영키 매핑 누락을 교차 확인했다.

## 대안

- 영문 문자면 조합을 해제: 영문 결과를 내는 다른 입력기를 손상시킬 수 있어 제외.
- 한영키를 검사: 해당 키가 UI 콜백으로 전달되지 않을 수 있어 제외.
- 창 프로시저 외부 가로채기: 재진입과 해제 관리가 추가되어 제외.
- 사용자 Cargo 캐시 직접 수정: 재현 가능한 배포가 아니므로 제외.

## 한계

원래 보고된 비정상 종료의 스택은 없다. 확인된 문자 손실 경로 및 그 후의 편집 안정성을 자동 검증하되 모든 종료 원인이 해결되었다고 단정하지 않는다.

## 실제 재현 재조사

`cargo run`으로 실행한 debug 빌드에서도 재현됨을 사용자가 확인했다. 오래된 release 빌드 가설은 이번 재현의 원인에서 제외한다. 실제 Windows 이벤트를 관측하기 전에는 종료 알림 누락만으로 원인을 확정하지 않는다.

## 실제 로그 분석과 결정

- 조합 영역 `1..2`, 네이티브 조합 길이 2바이트인 상태에서 `WM_CHAR`가 먼저 도착했다.
- 이때 `replace bytes=1 marked_before=Some(1..2)`가 실행되어 마지막 한글을 덮었다.
- 뒤늦은 `GCS_RESULTSTR`은 `marked_before=None`으로 3바이트 한글을 삽입했다.
- 종료 알림 선행 가정을 폐기한다. 일반 문자 삽입 전에 실제 네이티브 조합을 확정한다.
- API 근거: [Microsoft의 ImmNotifyIME 문서](https://learn.microsoft.com/en-us/windows/win32/api/imm/nf-imm-immnotifyime)의 CPS_COMPLETE는 조합 문자열을 결과 문자열로 설정한다.
- 단순 unmark는 입력기 내부 조합을 남겨 뒤늦은 결과를 중복 삽입하므로 사용하지 않는다.

## 한/영 전환 분석

- 플랫폼 공통 메시지 루프는 DispatchMessage만 호출한다. TranslateMessage는 각 키 경로의 책임이다.
- `handle_key_event`는 VK_PROCESSKEY를 원래 키로 복원한 뒤 일반 문자 매핑을 시도한다.
- 한/영키처럼 매핑되지 않는 키는 `None`이 되어 기존 key-down 함수가 TranslateMessage 없이 반환한다.
- 입력기 처리 키를 가장 먼저 OS 번역에 전달하고 앱 단축키 경로를 우회한다.
- 근거: [Microsoft ImmGetVirtualKey 문서](https://learn.microsoft.com/en-us/windows/win32/api/immdev/nf-immdev-immgetvirtualkey), [WM_KEYDOWN 문서](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-keydown). 원래 wParam을 바꾸지 않고 TranslateMessage에 전달한다.
