# 데이터 모델

ActionOptions::Push(bool)는 팝업별 옵션이다. false는 기존 Push, true는 PushWithLease 백그라운드 작업으로 변환된다. lease OID는 실행 시작 시 원격 추적 참조에서 읽으며 미존재는 zero다. 영속 상태는 없다.
