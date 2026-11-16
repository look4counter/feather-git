# 조사 결과

- 레인 계산은 feather-git-old/src/graph.rs의 GraphBuilder::append를 그대로 재사용한다. 고정 표시 순서에는 order_commits가 필요하지 않다.
- 실제 그리기는 feather-git-old/src/history_view.rs의 RowGraph이다. 배경 div 대신 PathBuilder로 세로선을 채우고 다른 레인은 cubic_bezier_to로 연결한다.
- 참조 theme.rs의 GRAPH 8색과 HEAD_FILL 흰색을 그대로 사용한다.
- 원본 행 높이는 24px이지만 이번 범위는 그리는 방식이므로 현재 행 38px을 공통 상수로 사용한다. 캐시 키는 위치·너비·레인 수·DPI이다.
- 단순 점/선 CSS식 div를 확장하는 대안은 곡선·DPI·캐시 방식이 달라 동일성 요구를 만족하지 않는다.

## 노드 선명도 조사 보완

GPUI 로컬 window.rs의 fill, PaintQuad::corner_radii, border_widths는 둥근 사각형과 내부 테두리를 지원한다. 정사각형 크기의 절반을 모서리 반경으로 주면 원이 된다. 기존 원의 중심은 선 두께의 홀짝에 맞춘 값으로 원 지름의 홀짝과 맞지 않을 수 있다. 노드는 별도 물리 경계 정렬을 사용한다.
