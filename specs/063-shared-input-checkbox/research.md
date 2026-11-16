# 조사 결과

기존 TextInput 및 모든 엔티티 표시 위치, 설정 체크박스와 Diff 기호를 조사했다. 기존 Selection::select_word는 단어를 영숫자·한글·밑줄, 공백 묶음, 기타 한 문자로 구분한다. GPUI index_for_x는 글자 시작 위치를 반환하므로 단어 경계 오른쪽 절반 클릭에서도 정확하다. 삭제 아이콘 시스템은 canvas 기반 PathBuilder이므로 동일 방식으로 작은 휴지통을 구현한다. 미해결 기술 쟁점 없음.
