# 구현 계획: Windows 크레이트 업데이트

**작성일**: 2026-11-05
**명세**: [기능 명세](spec.md)

## 요약

Cargo.toml의 직접 windows 의존성을 0.62.2로 변경하고 Cargo.lock을 갱신한다. 기능 플래그를 유지하며 Windows 빌드로 호환성을 확인한다.

## 기술 맥락

- 언어: Rust, 에디션 2024
- 주요 의존성: GPUI 0.2.2, windows 0.62.2
- 저장소: 기존 파일을 유지하며 데이터 변경 없음
- 검증: cargo check --locked --all-targets
- 대상: Windows 데스크톱
- 성능 목표: 기존 동작 유지
- 제약: 기존 사용자 수정 보존, 전이 의존성의 다른 windows 버전 허용
- 범위: Cargo.toml, Cargo.lock, 필요시 기존 Windows 호출부의 최소 호환성 수정

## 헌장 점검

헌장은 미작성 템플릿이므로 적용 가능한 추가 원칙이 없다. 사용자 지침의 한글 문서와 speckit 단계 준수를 확인했다. 설계 후에도 위반 없음.

## 조사 및 설계

[조사](research.md), [데이터 모델](data-model.md), [계약](contracts/compatibility.md), [검증 안내](quickstart.md)를 따른다.

## 프로젝트 구조

- Cargo.toml: 직접 의존성 선언
- Cargo.lock: 의존성 해석 결과
- src/auth_profiles.rs, src/repository_watch.rs, src/setup.rs: Windows 호출 호환성 확인 대상
- specs/011-update-windows-crate/: 명세, 계획, 작업 및 검증 기록

## 실행 단계

1. 현재 의존성 및 기존 변경을 확인한다.
2. 직접 의존성과 잠금 파일을 갱신한다.
3. Windows 대상 컴파일과 변경 범위를 검증한다.
4. 수렴 점검으로 누락된 요구사항이 없는지 확인한다.
