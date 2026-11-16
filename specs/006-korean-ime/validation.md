# 검증 결과: Windows 한글 조합 보존

## 원인과 old 비교

현재 배포 GPUI는 일반 WM_CHAR를 바로 replace_text_in_range로 전달하고 IME 종료를 처리하지 않아 남은 조합 범위를 영문으로 덮어쓴다. old의 007-fix-korean-ime 문서와 vendor 코드는 이 문제에 대한 실제 메시지 관측 및 최종 사용자 확인 기록을 포함한다.
동일 0.2.2 배포 소스를 로컬 고정하고 old의 Windows IME 최종 수정만 적용했다. vendor 파일을 원본과 해시 비교한 차이는 한글 README, Windows 모듈 등록, events.rs, window.rs 및 신규 ime.rs뿐이다. ime.rs 내용은 old 최종 파일과 일치한다. 진단 기록 코드는 제외했다.

## 수정 및 요구사항 대응

- FR-001/FR-004: WM_CHAR에서 네이티브 조합이 남았으면 입력 핸들러/창 상태를 빌리지 않고 CPS_COMPLETE를 요청한 뒤 문자를 삽입한다.
- FR-002: 종료 보류·늦은 결과·결과 후 새 조합·취소를 상태 전이로 처리한다.
- FR-003: 실제 IME 처리/모드 키를 Windows 번역으로 먼저 보내고 일반 Alt/오른쪽 Alt/문자/방향키 경로는 유지한다.
- FR-005: Cargo.toml/Cargo.lock은 같은 GPUI 0.2.2의 vendor 경로를 사용한다. 사용자 Cargo 캐시와 old는 변경하지 않았다. 다른 플랫폼 소스와 렌더링 파일은 배포 원본과 동일하다.
- FR-006: 실제 TextInput 콜백과 회귀 테스트가 같은 apply_edit 함수를 사용하며, 테스트는 실제 vendor IME 상태/순서 함수를 포함한다. 새 영문 판별 우회 로직은 없다.

## 재현과 자동화

- 수정 전 Windows 전달 순서 기준 구현에서 회귀 테스트 6개 중 5개 실패를 확인했다. 러l의 실제 결과는 l이었다. 한글 연속 조합 경계와 IME 모드 키 전달도 실패했다. dead-key는 기존 동작대로 통과했다.
- old 최종 상태 전이/순서 함수를 적용한 뒤 기대값과 테스트 시나리오를 수정하지 않고 모두 통과했다.
- cargo fmt --check: 통과.
- 변경한 vendor Windows 파일의 rustfmt --check --edition 2024 --config skip_children=true: 통과.
- cargo check --offline: 통과.
- cargo test --offline: 전체 39개 통과. IME 회귀 6개와 기존 저장소/폼/입력/그래프/탭/Alert 검증 포함.
- cargo clippy --offline --all-targets: 통과. 기존 탭 테스트의 field_reassign_with_default 경고는 유지된다.
- cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-ime.exe: 통과.
- cargo tree --offline -i gpui: GPUI 0.2.2가 작업 공간 vendor/gpui에서 연결됨을 확인했다.
- 배포 GPUI 원본 taffy.rs의 부동소수점 추론 미래 호환성 경고 2개와 기존 proc-macro-error2 미래 호환성 경고, 링크/출력 파일 안내가 있다. 무관한 소스 수정이나 경고 숨김은 하지 않았다.

## 미검증 범위

이전 화면 도구의 실행 파일 사용 승인 제한으로 현재 빌드의 실제 Windows 한/영 키 전환 및 러l 재입력은 직접 검증하지 못했다. 자동화에서 실제 편집 교체 함수와 네이티브 메시지 순서를 검증한 결과를 실제 IME 조작 확인으로 확대하지 않는다. old의 이전 사용자 확인은 이번 실행 파일의 직접 확인과 구분한다.
