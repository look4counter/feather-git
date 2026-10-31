# 구현 계획: Windows 실행 시 콘솔 창 숨기기

**작성일**: 2026-10-31  
**명세**: [spec.md](spec.md)

## 요약

Rust 주 실행 파일에 Windows GUI 서브시스템 속성을 적용한다. Windows 이외 플랫폼에서는 조건부 속성이 적용되지 않는다.

## 기술 배경

- 언어: Rust 2024
- 대상: Windows 데스크톱 앱
- 주요 파일: `src/main.rs`
- 검증: `cargo check`, Windows 빌드와 실행 파일 직접 실행
- 데이터 모델 및 외부 계약: 해당 없음

## 헌법 확인

헌법은 미작성 템플릿이다. 저장소 지침에 따라 한글 문서와 전체 speckit 절차를 따른다.

## 구현 방식

`src/main.rs` 크레이트 루트에 `#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]`를 추가한다. 이 속성은 Windows 실행 파일의 서브시스템을 GUI로 지정한다. 의존성 및 기존 창 생성 코드는 수정하지 않는다.

## 검증 방식

Rust 검사로 구문과 조건부 속성 적용을 확인한다. 가능한 경우 Windows 실행 파일을 빌드하여 PE 서브시스템 값을 확인하고 실행 시 창 동작을 확인한다.
