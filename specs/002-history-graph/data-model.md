# 데이터 모델

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

- 샘플 Commit: 기존 필드와 부모 해시 목록 parent_ids. 표시 순서는 자식이 부모보다 앞이다.
- GraphCommit: id, parent_ids, marker, input_edges, output_edges. UI 및 저장소 접근과 무관하다.
- GraphMarker: lane과 color 인덱스.
- GraphEdge: 기다리는 부모 해시, 출발·도착 레인, 색상과 tip 여부.
- Workspace: 렌더링할 RowGraph 목록과 이력 전체의 최대 레인 수. 선택 인덱스는 기존대로 유지한다.
- RowGraph: 선분, 노드 레인·색 및 공유 경로 캐시. 렌더링 시 입력 데이터는 불변이다.
