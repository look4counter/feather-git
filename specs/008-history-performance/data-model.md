# 데이터 모델

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

HistoryState는 GraphCommit 토폴로지, 레인 수, 선택적 준비 행 목록과 UniformListScrollHandle을 보관한다. PreparedHistoryRow는 SharedString 표시값과 RowGraph를 공유해 반복 렌더링 시 문자열/그래프를 재준비하지 않는다. Workspace는 프로젝트 인덱스별 캐시를 저장하고 스냅샷 교체·탭 닫기에만 무효화한다.
