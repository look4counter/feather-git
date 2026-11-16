# 조사 결과

- old/specs/007-fix-korean-ime/quickstart.md에는 2026-11-03 최종 사용자 실제 입력 확인이 있다.
- old와 현재 배포 events.rs 비교에서 문자 입력 전 네이티브 조합 확정, 종료 상태 처리, 결과 우선 순서 및 입력기 키 번역이 빠져 있음을 확인했다.
- old의 window.rs 차이는 IME 상태 필드/초기화뿐이며 events.rs 차이도 IME 및 진단이다. 진단 코드는 이번에 제외한다.
- 영문이면 무조건 unmark하는 방식은 늦은 결과 중복과 dead-key 손상 위험이 있어 사용하지 않는다.
- 같은 0.2.2 소스를 vendor로 고정하면 Cargo 캐시 변경 없이 재현 가능한 빌드가 된다. 원본 코드와 LICENSE-APACHE를 보존한다.
