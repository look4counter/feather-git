# 상태 모델

VirtualList의 fixed_left_width는 기본 0인 선택적 렌더 설정이다. 실제 왼쪽 inset은 뷰포트 유효 폭으로 제한한다. 본문 폭은 전체 최소 콘텐츠 폭에서 실제 inset을 뺀 값이며 음수가 되지 않는다. DiffViewer의 기존 text_left는 일반 diff 116px, 부분 Stage diff 140px이며 상세 화면은 고정 모드가 아니다. 선택 위치와 스크롤 핸들 상태 형식은 유지한다.
