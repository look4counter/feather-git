# 구현 계획: 빌드 경고 정리

**기능**: `008-clean-build-warnings` | **날짜**: 2026-11-03 | **명세**: [spec.md](spec.md)

## 요약

GPUI 수치 리터럴에 f32를 명시하고, proc-macro-error2 2.0.1의 proc_macro 공개 범위를 바로잡는다. MSVC 실행 파일의 불필요한 import library 생성을 /NOIMPLIB와 /NOEXP로 중단하여 정보성 링커 출력을 없앤다. 실제 경고 전역 차단은 하지 않는다.

## 기술 배경

- 언어: Rust 2024, rustc/cargo 1.98.0.
- 플랫폼: Windows MSVC 데스크톱.
- 의존성: 로컬 GPUI 0.2.2, proc-macro-error2 2.0.1.
- 저장 데이터 변경 없음. 기존 한글 입력 수정 보존.
- 검증: 개발·배포 빌드 출력, 기존 입력 회귀 15개, PE 내보내기 이름 비교.
- 성능: 실행 시 추가 작업 없음.

## 헌장 확인

헌장은 미작성 템플릿이며 별도 규범이 없다. 한글 문서 규칙과 speckit 단계를 준수한다. 계획 전·후 위반 없음.

## 구조와 수정 범위

- `vendor/gpui/src/taffy.rs`: 타입 추론에 의존한 리터럴을 명시적 f32로 변경한다.
- `vendor/proc-macro-error2/`: 기존 2.0.1 소스와 라이선스를 보존하고 proc_macro 선언에 pub을 추가한다. Markdown 설명은 한글로 제공한다.
- `Cargo.toml`, `Cargo.lock`: 재현 가능한 로컬 패치 연결.
- `build.rs`: 대상이 Windows MSVC일 때만 /NOIMPLIB와 /NOEXP를 전달한다. 링크 오류와 실제 진단을 억제하지 않는다.
- `vendor/gpui/README.md`: 변경 출처 기록.

## 조사와 설계

[research.md](research.md)에 경고 원인과 선택 근거를 정리한다. 새 사용자 데이터 개체나 외부 인터페이스는 없다. 기존 빌드 계약과 플랫폼 구분은 [data-model.md](data-model.md)에 정의하고 검증 절차는 [quickstart.md](quickstart.md)에 둔다.

기존 실행 파일의 export는 libgit2의 git_* 항목이며 import library 출력과 별개다. 두 옵션 적용 전후 이름 목록을 비교하여 보존을 확인한다. 기존 캐시를 고치거나 의존성을 무관하게 갱신하지 않는다.

## 단계

1. 원래 빌드 경고와 실행 파일 export 목록 기록.
2. 의존성 소스와 플랫폼별 빌드 옵션 최소 수정.
3. 빌드·입력 회귀·export 비교 후 수렴 점검.

## 복잡도 검토

작은 의존성 하나를 로컬에 보존하여 한 줄의 향후 호환성 수정을 재현한다. 새 외부 라이브러리나 런타임 분기를 추가하지 않는다.
