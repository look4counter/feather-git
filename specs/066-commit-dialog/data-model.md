# 데이터와 상태

CommitDraft는 HEAD/참조/일반 기본 메시지/마지막 메시지/스테이지 여부/정정 가능 여부이다. CommitRequest는 기대 HEAD와 참조, 메시지, push/amend 옵션이다. CommitOutcome은 생성된 commit id와 선택적 error이다. CommitWorkResult는 실행 결과와 별도 스냅샷 재조회 결과를 갖는다. UI의 메시지 초안과 옵션은 팝업 수명 안에서만 유지하며 세션 형식은 변경하지 않는다.
