# 조사 결과

기존 스태시 객체 스냅샷과 선택 검증을 재사용하면 다른 인덱스 항목을 적용하는 위험을 줄일 수 있다. 별도 적용 팝업·복원 옵션은 추가하지 않는다. refs/stash 잠금은 별도 저장소 핸들에 보관하여 mutable stash_apply와 함께 유지한다. 적용은 refs/stash를 수정하지 않는다. git2 StashApplyOptions의 안전한 checkout 옵션을 사용하고 index 재복원은 요청하지 않는다. 미해결 기술 질문 없음.
