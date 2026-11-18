# 조사 결과

결정: libgit2의 기존 인증 및 push_negotiation을 재사용한다. PushUpdate.src는 서버 현재 값이고 dst는 새 값이다. 기존 원격 fetch refspec으로 추적 값을 구해 고정된 lease와 비교한다. 비교 없는 +refspec과 전송 직전 자동 fetch는 보호 의미를 훼손하므로 제외한다. CLI 별도 인증 경로 도입도 제외한다.

근거: [Git Push 문서](https://git-scm.com/docs/git-push), [git2 PushUpdate 문서](https://docs.rs/git2/0.21.0/git2/struct.PushUpdate.html), [git2 Refspec 문서](https://docs.rs/git2/0.21.0/git2/struct.Refspec.html).
미해결 기술 질문은 없다.
