# GPUI 0.2.2 로컬 패치

현재 사용 중인 배포 GPUI 0.2.2의 소스·리소스와 LICENSE-APACHE를 보존한 복사본이다. 원본은 https://crates.io/crates/gpui/0.2.2 이며, 저작자 정보는 Cargo.toml에 유지한다.

Windows IME의 종료·확정 순서와 한/영 전환 키 처리를 feather-git-old의 최종 조치에 맞춰 수정한다. 다른 플랫폼/렌더링 및 공개 API는 유지하고 진단 로깅은 추가하지 않는다. 변경 근거와 검증은 specs/006-korean-ime에 기록한다.
