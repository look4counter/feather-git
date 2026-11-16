# 데이터 모델

> 정리 안내: 단계별 설계·작업 기록이며 후속 변경은 [현재 명세 색인](../README.md)과 해당 기능의 spec.md를 따른다. 기존 체크 상태와 과거 검증 이력은 현재 재검증 완료를 의미하지 않는다.

Diff의 eof_rows는 실제 파일 끝을 포함하는 기존 diff 행 인덱스 집합이며 empty_eof는 빈 텍스트 파일 표시 여부다. 기존 DiffLine.text·Selection·부분 Stage Preview는 변경하지 않는다. 파일 갱신 요청은 기존 PanelRequest를 사용하고 Stage 경로에서 graph가 None이다.

F5는 기존 PanelRequest와 ViewerChanged만 사용한다. 새 영속 상태는 없으며 요청 revision·project·path로 오래된 결과를 거부한다.
