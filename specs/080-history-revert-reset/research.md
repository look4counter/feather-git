# 조사 결과

Git 공식 revert/reset 문서를 확인했다. no-commit은 커밋을 만들지 않고 반대 변경을 인덱스·작업트리에 적용한다. reset 기본값은 Mixed이며 Hard는 추적 작업 파일을 덮어쓰고 방해하는 미추적 경로도 표준 Git 의미로 제거할 수 있다. 팝업에 영향을 명시한다. 병합 Revert에는 mainline이 필요하므로 첫 부모를 명시적으로 사용한다.

기존 Git 계층은 libgit2와 일부 숨긴 Git CLI를 함께 사용한다. 새 작업은 표준 Git CLI 의미를 재사용하여 커밋 생성·루트 커밋·충돌 상태를 일관되게 처리한다. 인덱스 잠금은 Git에 맡긴다. 기존 with_locked_index를 CLI 바깥에 걸면 Git 자체 잠금과 충돌하므로 사용하지 않는다.

확인창은 기존 modal 차단과 동일한 수명으로 관리하고 확인 후 기존 실행 함수를 호출한다. 그래프 HEAD 표시 건은 사용자가 착각이라고 철회했으므로 코드와 테스트를 변경하지 않는다.

참고: [Git Revert 문서](https://git-scm.com/docs/git-revert), [Git Reset 문서](https://git-scm.com/docs/git-reset).

## old 찾기 분석

읽은 원본은 D:/Projects/feather-git-old/src/git_search.rs와 src/ui.rs의 run_search·activate_search_result·open_search_file·search_panel 및 기존 Space/O 처리다. 검색 의미는 모든 heads/remotes/tags·HEAD의 커밋 메시지 본문, 과거 파일 경로와 UTF-8 비바이너리 내용, 인덱스와 미추적 파일의 현재 내용을 대소문자 무시로 검색하는 것이다. 기존 단축키 작업은 새 결과 컨텍스트 메뉴로 옮겼다.

파일 결과는 현재 앱의 FileHistoryDialog를 재사용한다. 기록에 없는 커밋은 새 commit_snapshot이 기존 snapshot_with_target을 재사용해 읽기 전용으로 찾는다. 검색 콘텐츠 경로는 정규화한 root 하위의 일반 파일만 읽으며 부모·절대 경로·누락·디렉터리 입력을 검증한다. 취소는 명시적 tree 스택의 반복 경계에서 처리한다.

전역 F5는 기존 조회·진행·성공/오류 갱신을 재사용하는 Refresh 작업으로 구현했다. Fetch 함수와 원격 연결을 호출하지 않으며 동일 뷰어 대상도 강제 재조회한다.

## Revert 계속 조사
Git 공식 문서와 임시 저장소 실험으로 --continue·--abort·--quit을 확인했다. 단일 no-commit 충돌의 --continue는 새 커밋을 만들기 때문에 앱의 원래 옵션 문맥을 보존한다. --quit은 해결된 작업트리·인덱스를 유지하면서 진행 정보를 지우므로 no-commit의 완료에 사용한다. TextInput은 모든 편집 입력이 공유하며 line_bounds/select_to를 이미 제공하지만 Shift+Home/End action과 바인딩이 없다.

## 그래프 중복 부모 레인 선택 원인

기존 코드는 입력 간선을 `(waiting_commit_id, edge)` 쌍으로 HashMap에 collect했다. 같은 부모 ID를 가진 입력이 여러 개일 수 있는데 단일 값 맵으로 만들면서 나중 입력이 앞선 입력을 덮어썼다. 입력은 레인 순서로 처리되므로 결과적으로 오른쪽 레인이 남는다. 의도적으로 마지막 레인을 선택해야 하는 정책이나 이 상황을 검증하는 기존 테스트는 없었다. 실제 7817dfdf 행에는 703b5ac3를 기다리는 레인 0과 레인 6이 있고, 기존 코드는 레인 6을 골랐다.

앞선 수정은 이를 레인 공유 자체의 문제로 해석해 새 레인 7을 만들었다. 사용자는 첫 번째 레인이 정답이라고 정정했으므로 해당 해석과 독립 레인 수용 기준을 폐기했다. 최종 정책은 추가 부모의 기존 입력 중 최소 to_lane과 해당 색을 재사용하는 것이다. 기존 후보가 없을 때만 새 레인을 만들고 첫 부모는 현재 노드 레인을 유지한다. min_by_key로 기준을 명시하여 HashMap의 중복 키 처리나 우연한 반복 순서에 의존하지 않는다.
