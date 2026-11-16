# 상태 모델

파일 이력 항목은 Commit과 해당 시점 FileChange다. 조회는 revwalk의 현재 커밋과 요청 경로의 전후 tree 항목만 사용한다. rename 연결/경로 별칭 상태는 제거한다. 팝업은 항목/표시용 Project·HistoryState·독립 선택·읽기 취소 슬롯·자식 DiffViewer·포커스와 닫기 이벤트 구독을 보유한다. 영속 모델은 변경하지 않는다.
