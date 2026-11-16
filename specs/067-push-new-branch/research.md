# 조사 결과

기존 push_current의 전송 콜백을 재사용한다. Commit 즉시 푸시의 정책은 바꾸지 않고 새 Push 작업만 성공 후 upstream을 설정한다. 새 브랜치의 시작점은 현재 HEAD이므로 생성 후 같은 커밋을 가리키는 HEAD를 전환하면 checkout_tree 없이 미스테이지·스테이지·미추적 파일을 그대로 보존할 수 있다. git2 Branch::name_is_valid와 force=false로 이름/중복을 검증한다. UI와 IO를 섞지 않는다.
