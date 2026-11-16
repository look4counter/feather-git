# 실행 및 검증

cargo fmt --check, cargo check --offline, cargo test --offline, cargo clippy --offline --all-targets를 실행한다. cargo rustc --offline --bin feather-git -- -o target/debug/feather-git-progress.exe로 빌드한다.
새 탭 저장소 설정에서 탐색은 별도 진행 문구 없이 폴더 선택 창을 열어야 한다. 확인으로 실제 Git 작업을 수행할 때는 기존 폼 영역에 공통 깃털/물결 애니메이션이 표시되어야 한다. 오류 Alert와 작업 후 폼 복귀도 확인한다.

## 다른 화면에서 Dialog 재사용

타이틀바와 분리된 본문 호스트에 아래처럼 Dialog를 넣는다. 제목·본문·버튼·너비와 키보드 콜백을 호출부가 지정할 수 있다. 표시 여부와 포커스 수명은 호출부에서 관리한다.

```rust
Dialog::new("custom-dialog", "사용자 지정 팝업")
    .width(px(480.))
    .body(div().child("여기에 업무별 내용을 구성합니다."))
    .footer(div().child("버튼 영역"))
```

RepositoryDialog는 저장소 입력과 확인/취소를, Alert는 메시지와 확인 버튼을 같은 기반 위에 구성한다. 모든 팝업과 중첩 오류 Alert가 본문 안에 있는지, 타이틀바 배경/이동/창 제어가 가려지지 않는지 크기 변경과 함께 확인한다.
