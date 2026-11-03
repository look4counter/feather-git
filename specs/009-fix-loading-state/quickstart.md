# 검증 절차

## 자동 검증

저장소 루트에서 실행한다.

```powershell
cargo test --offline --lib reload_tests
cargo test --offline
cargo build --offline --bin feather-git
cargo fmt --all -- --check
```

회귀 검증에서 두 완료 순서와 중첩 요청, 정상·오류 완료의 상태 정리가 통과해야 한다.

## 화면 확인

1. 저장소의 Search 탭에서 커밋 검색 결과를 준비한다.
2. 새로고침 도중 키보드로 검색 결과 커밋을 선택한다.
3. 조회 완료 후 차단 표시가 사라지고 선택 커밋이 유지되는지 확인한다.
4. 히스토리에서 태그·병합 창을 열고 취소하며 설정도 열리는지 확인한다.
5. 조회가 빠르면 큰 저장소 또는 디버거로 완료 순서를 제어한다. 이 순서 제어는 자동 모델 회귀 검증으로도 확인한다.

