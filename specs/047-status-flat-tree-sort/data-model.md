# 데이터 모델

기존 FileChange의 staged·status·path를 그대로 사용한다. 정렬 결과는 표시 모델과 diff 선택의 공통 기준이다. 새 영속 상태나 폴더 노드는 추가하지 않는다.

2026-11-16: 그룹 내부 상태 순위는 U → M → D → A → 기타이며 staged·status·path 및 충돌 분류는 유지한다.
