# 상태 모델

FileChange의 staged와 path를 기존 형태로 사용한다. 조회 결과 순서가 표시 목록과 diff 선택의 공통 기준이다. BTreeSet 기반 checked는 기존 행 인덱스 집합을 유지하며 구간 선택은 해당 구간의 추가·삭제 행만 변경한다. 새 영속 데이터는 없다.
