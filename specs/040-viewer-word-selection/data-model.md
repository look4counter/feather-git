# 상태 모델

Selection의 anchor와 cursor는 기존 행·문자 인덱스를 유지한다. 단어 선택은 같은 행의 시작과 끝으로 두 위치를 설정한다. 새 영속 상태는 없다. 기록 재선택은 file_history·project·commit을 읽으며 기존 조회 및 선택 상태를 변경하지 않는다.
