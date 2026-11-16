# 조사 결과

rg 호출 검색과 Cargo 호환성 보고서를 확인했다. proc-macro-error2 경고는 vendor/proc-macro-error2/src/lib.rs의 private extern crate proc_macro를 __private 모듈에서 pub use하는 데서 발생한다. 같은 버전의 선언 공개로 해결 가능하다. 추가 조사나 모호성 없음.
