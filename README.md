# Lightweight Source Parser

가볍고 범용적인 문서 Source Parser.

목표는 문서를 의미적으로 해석하는 것이 아니라, 후속 시스템이 원문을 안정적으로 조회·재조회·검증할 수 있도록 구조와 위치를 보존한 `Canonical Source + Stable Anchor`를 만드는 것이다.

## Core Principle

**Deterministic First, Intelligence on Failure**

- DOCX 등 구조가 명시된 입력은 deterministic하게 처리한다.
- Digital PDF는 우선 text/position 기반으로 가볍게 추출한다.
- 구조 이상은 deterministic integrity check로 감지한다.
- 문제가 확인된 영역만 targeted recovery에 넘긴다.
- recovery 결과는 재검증 후에만 Canonical Source로 승격한다.

## Output

```text
Canonical Source
+
Stable Anchor
```

## Processing Flow

```text
Input
→ Lightweight Extraction
→ Structure Integrity Check
   ├─ VALID → Canonicalization
   └─ AMBIGUOUS / PARTIAL
        → Targeted Recovery
        → Re-validation
→ Canonical Source
→ Stable Anchor
→ Source Resolve
```

## Initial Scope

- DOCX deterministic extraction
- Digital PDF text/position extraction
- Canonical Source
- Stable Anchor
- Structure Integrity Validation
- Targeted Recovery
- Recovery Re-validation

## Out of Scope

- 문서 의미판단
- 논증 판단
- 도메인 inference
- 전체 문서 LLM parsing
- 자체 PDF rendering engine
- 자체 OCR 모델
- 불필요한 Agent orchestration

## Implementation

- Core language: Rust
- 외부 DOCX/PDF extractor와 recovery provider는 교체 가능한 adapter로 취급한다.
- 정상 deterministic path에서는 모델 호출을 하지 않는다.

## Development Status

현재 첫 세로절단:

```text
DOCX
→ deterministic extraction
→ Canonical Source
→ Stable Anchor
→ source resolve
```

## Validation

기본 검증:

```bash
cargo fmt --check
cargo test
```

구조 보존, anchor roundtrip, failure separation을 fixture 기반으로 검증한다.
