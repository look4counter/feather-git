# 데이터 모델

CommitReference는 원래 shorthand 이름 name, 표시 이름 label, 원격 여부 is_remote를 가진다. 원격 여부는 is_remote()만 결정한다.
Commit.references는 참조별 메타데이터이며 기존 Commit.reference는 결합된 원래 이름을 유지한다.
PreparedReference는 SharedString 표시 이름과 원격 여부이다. PreparedHistoryRow.references는 Rc<Vec<PreparedReference>>로 공유한다. 메타데이터가 없고 기존 문자열이 있으면 문자열 그대로의 비원격 배지를 만든다.
