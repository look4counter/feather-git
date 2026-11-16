# 조사 결과

- Git 읽기 전용 status의 종료 코드는 0이며 변경 항목은 0개다.
- gix 0.90.0의 status/index_worktree.rs에서 summary()는 NeedsUpdate에 None을 반환한다. 현재 앱은 해당 항목도 M으로 변환한다.
- 초기 가설: summary가 없는 IndexWorktree 항목 제외를 검토했다. 실제 재현에서는 유지보수 항목이 없어 채택하지 않았다. 모든 M을 내용 비교로 재검사하거나 CRLF를 임의로 무시하는 대안은 실제 변경을 숨기고 중복 비용을 만들 수 있어 제외한다.
- 결정: 사용자 저장소는 읽기 전용으로 검증한다. 실제 앱을 맞추기 위해 git add나 인덱스 refresh를 실행하지 않는다.

## 재현 결과 보완

실제 저장소의 gix는 core.autocrlf=None, libgit2는 true였다. 설치 설정 경로는 Git의 config --show-origin과 libgit2::Config::find_system이 일치했다. 유지보수 항목 가설은 실제로 0건이므로 이번 수정에서는 적용하지 않는다. libgit2 기본 설정의 안전한 줄바꿈 세 값만 상속하는 방식을 채택한다.
