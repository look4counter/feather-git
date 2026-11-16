# 구현 계획: Windows 한글 조합 보존

## 분석 결과

현재 배포 GPUI 0.2.2의 Windows events.rs는 WM_IME_ENDCOMPOSITION을 처리하지 않고 WM_CHAR를 곧바로 replace_text_in_range(None, 문자)로 전달한다. TextInput은 남은 marked_range를 우선 교체하므로 러가 l로 바뀐다. IME 모드 키도 일반 키 매핑 실패 시 TranslateMessage에 도달하지 않을 수 있다.
old의 specs/007-fix-korean-ime와 vendor 변경을 비교했다. Windows events.rs/window.rs/모듈 등록 및 신규 ime.rs가 최종 조치이고, 별도 ime_trace는 진단 전용이다. old는 실제 사용자 입력 확인까지 기록했다. 다른 플랫폼/그래프/입력 UI 변경은 필요 없다.

## 설계와 영향 범위

- vendor/gpui: 현재 Cargo 캐시의 동일한 0.2.2 배포 소스를 작업 공간에 복사한다. 빌드 소스/리소스와 Apache 라이선스를 보존하고, 필요 없는 문서 원문은 복사하지 않는다. README는 한글로 출처와 패치 범위를 기록한다.
- Windows ime.rs의 상태 전이와 key/character 순서 함수, events.rs의 메시지 처리, window.rs의 IME 상태 필드, windows.rs 모듈 등록에 old 수정만 적용한다. 진단 로그는 이식하지 않는다.
- WM_CHAR 전에 네이티브 조합 존재를 확인하고 ImmNotifyIME의 CPS_COMPLETE를 호출한다. 동기 확정 메시지 재진입 동안 창/입력 핸들러를 빌리지 않는다. 종료와 결과 순서, 이전 결과와 새 조합, 취소는 상태 전이로 처리한다.
- Cargo.toml의 patch.crates-io와 Cargo.lock으로 동일 버전 로컬 패치를 고정한다. 다른 의존성은 교체하지 않는다.
- src/text_input.rs의 기존 교체 로직을 공통 순수 함수로 추출해 실제 IME 콜백과 회귀 테스트가 같은 교체 코드를 사용한다. 영문 판별 우회는 넣지 않는다. unmark 시 화면 갱신도 수행한다.
- src/ime_tests.rs는 실제 vendor ime.rs와 TextInput 교체 함수를 사용한다. 현재 Windows 전달 동작의 기준 구현에서 회귀 실패를 확인한 뒤 동일 테스트를 수정 없이 최종 상태 전이에 적용한다.

## 헌장·안전·호환성

헌장은 미작성 템플릿으로 검사 생략. GPUI 버전과 API, 앱 데이터/모듈 책임을 유지한다. 로컬 패치는 기존 핵심 라이브러리의 Windows 오류 수정이며 다른 라이브러리로 교체하지 않는다. 사용자 Cargo 캐시와 old는 읽기만 한다. 입력 본문/진단 로그와 프로세스 외부 가로채기는 추가하지 않는다.

## 단계 및 검증

1. 기존 교체 로직 추출과 실제 이벤트 순서 재현 테스트를 준비한다. 러l 손실과 IME 전환 키 전달 실패를 확인한다.
2. old의 최종 IME 패치를 적용하고 Cargo 로컬 경로를 확인한다.
3. 러l/쿼리ID, 한글·이모지 접두어, END/RESULT 순서, 결과+새 조합, 취소·새 경계, dead-key, 일반 Alt/방향키 및 조합 뒤 편집을 검증한다.
4. cargo fmt --check, 변경 vendor 파일 rustfmt 검사, cargo check/test/clippy --offline, 실행 파일 빌드 및 cargo tree로 로컬 연결을 확인한다. 실제 Windows IME 입력 여부는 별도 명시한다.
