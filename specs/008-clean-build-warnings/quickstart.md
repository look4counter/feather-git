# 검증 절차

저장소 루트에서 다음을 실행한다.

```powershell
cargo build --offline --bin feather-git
cargo build --offline --release --bin feather-git
cargo test --offline --bin feather-git commit_editor::tests
```

알려진 경고 및 향후 호환성 안내 없이 성공해야 하며 입력 검증은 15개 모두 통과해야 한다. 각 실행 파일의 export 이름 목록을 수정 전과 비교한다. 캐시만 재사용한 성공이 아닌 수정한 의존성의 재컴파일 결과를 확인한다.

## 검증 결과 — 2026-11-03

- GPUI의 f32 fallback 경고 2개를 명시적 수치 타입으로 해결했다.
- proc-macro-error2 2.0.1의 비공개 외부 크레이트 재공개 문제를 수정했다. 원본과 비교하면 README 설명과 외부 크레이트 선언 한 줄만 달라졌고 라이선스는 동일하다.
- MSVC의 .lib/.exp 생성 안내는 실제 산출물 생성을 생략해 제거했다. 경고를 전역 차단하지 않았다.
- 개발용·배포용 실행 파일의 export 이름 목록은 수정 전후 각각 950개이며 차이는 0개다.
- 입력 회귀 검증 15개가 모두 통과했으며 검증 빌드에도 경고가 없었다.
- 배포용 빌드의 `--future-incompat-report` 결과는 `0 dependencies had future-incompatible warnings`였다.
- 로그는 `.build/warnings-debug-build.log`, `.build/warnings-release-build.log`, `.build/warnings-input-tests.log`에 보관한다. export 전후 목록도 `.build/warnings-*-exports-*.txt`에 보관한다.
- 기존 THIRD_PARTY_NOTICES와 THIRD_PARTY_LICENSES에 같은 버전의 의존성과 라이선스가 이미 포함되어 있다.
