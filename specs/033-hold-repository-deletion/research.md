# 조사 결과

GPUI는 on_mouse_down, on_mouse_up_out, on_hover와 on_next_frame·request_animation_frame을 제공한다. 기존 클릭 콜백에 타이머만 추가하면 손을 뗀 후에도 삭제할 위험이 있으므로 세대 번호로 완료 예약을 무효화한다. 완료 콜백은 가득 찬 프레임 다음에 실행한다. 창 활성과 팝업 포커스를 프레임마다 확인한다.

GPUI의 hover 판정은 pending_mouse_down이 없어야 참이므로 누름 중 이탈 판정에 사용할 수 없다. 실제 버튼 영역을 기준으로 전역 캡처 단계의 마우스 이동·창 이탈을 검사하고 활성화·포커스 구독으로 프레임이 정지된 동안의 취소도 반영한다.
