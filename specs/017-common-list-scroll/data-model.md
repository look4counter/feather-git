# 데이터 모델

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

Visibility는 가로·세로 넘침 여부이다. Thumb는 시작·길이·이동 가능 거리·최대 오프셋을 가진다. ScrollDrag는 축과 손잡이 내부 잡은 위치를 가진다.
ListScrollHandle은 세로 UniformListScrollHandle, 가로 ScrollHandle, 측정 프레임/트랙과 현재 호버·드래그를 공유한다. 프로젝트·패널 탭별 인스턴스를 분리한다. HistoryState의 기존 핸들은 이 내부 핸들과 동일하다.
VirtualList는 목록·선택 헤더·최소 콘텐츠 폭·공유 핸들을 가진다. 목록 감소·크기 변경·축 소멸 시 범위를 제한하고 드래그를 종료한다.
