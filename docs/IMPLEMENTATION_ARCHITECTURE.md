# 경량 범용 Source Parser 구현 아키텍처 v0.1

## 문서 메타데이터

- **Title:** Lightweight General-Purpose Source Parser Implementation Architecture
- **Document ID:** `SOURCE-PARSER-IMPLEMENTATION-ARCHITECTURE`
- **Design Baseline:** `SOURCE-PARSER-DESIGN-BASELINE v0.1`
- **Subject:** Deterministic extraction + integrity validation + targeted recovery 구현 구조
- **Scope:** 모듈 책임, 데이터 흐름, 인터페이스, 상태, 오류, 검증 구조
- **Out of Scope:** 제품 의미판단, 도메인 inference, 특정 외부 라이브러리 영구 고정
- **Version:** 0.1
- **Status:** draft
- **Created:** 2026-11-18

# 1. 구현 목적

입력 문서를 가능한 한 저비용 deterministic 경로로 처리하고, 구조 이상이 확인된 영역에만 지능 복구를 적용하여 다음 두 결과를 생성한다.

```text
Canonical Source
+
Stable Anchor
```

구현은 다음 원칙을 보존한다.

```text
정상 영역
→ deterministic

이상 영역
→ detect
→ targeted recovery
→ re-validation

모든 최종 Source
→ 원본에 재결속 가능
```

# 2. 전체 런타임 구조

```text
Input Document
      ↓
[1] Input Inspector
      ↓
[2] Format Adapter
      ↓
[3] Raw Source Graph
      ↓
[4] Structure Integrity Validator
      │
      ├─ VALID
      │    ↓
      │ Canonicalizer
      │
      └─ AMBIGUOUS / PARTIAL
           ↓
        Recovery Router
           ↓
        Targeted Recovery
           ↓
        Re-validator
           ↓
        Canonicalizer
              ↓
[5] Canonical Source
      ↓
[6] Stable Anchor Builder
      ↓
[7] Source Resolver
```

`FAILED` 또는 재검증 불가능 영역은 정상 Source로 승격하지 않는다.

# 3. 물리 책임

최소 구현 책임은 8개다.

```text
R1 Input Inspector
R2 Format Extraction Adapter
R3 Raw Source Graph
R4 Structure Integrity Validator
R5 Recovery Router
R6 Recovery Adapter
R7 Canonicalizer + Anchor Builder
R8 Source Resolver
```

각 책임은 반드시 별도 프로세스나 서비스일 필요가 없다.

초기 구현에서는 단일 라이브러리 내부 모듈로 구성할 수 있다.

# 4. R1 — Input Inspector

## 책임

입력을 해석하기 전에 파일의 기본 정체성과 처리 가능 여부를 판정한다.

최소 입력:

```text
bytes
filename
```

최소 출력:

```text
document_id
content_hash
detected_format
input_status
```

상태:

```text
SUPPORTED
UNSUPPORTED
DAMAGED
ENCRYPTED
```

# 5. R2 — Format Extraction Adapter

포맷별 저수준 추출 계층이다.

```text
FormatAdapter
├─ DocxAdapter
├─ PdfAdapter
└─ future adapters
```

Adapter는 교체 가능해야 한다.

외부 라이브러리의 타입과 ID가 상위 계층으로 누출되지 않게 한다.

# 6. DOCX Adapter

DOCX는 가능한 한 문서 내부 구조를 직접 사용한다.

추출 대상:

```text
section
heading
paragraph
list
table
row
cell
merge relation
caption
footnote
document order
```

가능한 경우 XML 내부 관계와 원문 구조를 직접 보존한다.

DOCX Adapter는 Canonical Source를 직접 만들지 않는다.

출력은 공통 `Raw Source Graph`다.

# 7. PDF Adapter

Digital PDF에서는 우선 최소 primitive만 추출한다.

```text
page
text span
bounding box
font/layout hints
image region
drawing region
```

PDF Adapter가 임의로 논리적 의미를 확정하지 않는다.

# 8. Raw Source Graph

모든 adapter의 출력은 하나의 중간 표현으로 통일한다.

예시:

```text
RawDocument
├─ RawPage
│  ├─ TextSpan
│  ├─ Region
│  ├─ Line
│  ├─ ImageRegion
│  └─ TableCandidate
└─ StructuralRelations
```

최소 공통 필드:

```text
raw_node_id
node_kind
content
source_position
parent_hint
order_hint
relation_hints
extraction_confidence
```

`Raw Source Graph`는 아직 Canonical Source가 아니다.

# 9. R4 — Structure Integrity Validator

이 계층이 구조 이상 여부를 deterministic하게 판정한다.

출력 상태:

```text
VALID
AMBIGUOUS
PARTIAL
FAILED
```

판정 결과는 최소한 다음을 포함한다.

```text
status
reason_codes[]
affected_regions[]
evidence[]
```

# 10. Integrity Rule 영역

## 10.1 Reading Order

검사 예:

- column crossing
- header/footer insertion
- spatial order contradiction
- duplicated spans
- impossible overlap

## 10.2 Table

검사 예:

- row alignment
- column alignment
- cell overlap
- missing cell relation
- header/data mismatch
- suspected merged cell loss

## 10.3 Caption

검사 예:

- caption 존재 + target 부재
- figure/table number mismatch
- 비정상적 거리

## 10.4 Footnote

검사 예:

- marker 존재 + body 부재
- body 존재 + marker 부재

## 10.5 Equation

검사 예:

- symbol ordering anomaly
- baseline/superscript relation loss
- fragmented expression

# 11. Integrity Rule 설계 원칙

각 rule은 독립적으로 실행 가능해야 한다.

형식:

```text
Rule
Input:
  raw nodes / relations

Output:
  PASS
  AMBIGUOUS
  FAIL

Evidence:
  node ids
  positions
  reason code
```

Rule은 문서 의미를 판단하지 않는다.

# 12. R5 — Recovery Router

Recovery Router는 복구 필요 여부만 결정한다.

입력:

```text
integrity findings
affected regions
input format
recovery availability
```

출력:

```text
NO_RECOVERY
RECOVERY_REQUIRED
RECOVERY_UNAVAILABLE
```

모델이 필요한지 판단할 때 문서 전체를 기본 전달 대상으로 삼지 않는다.

# 13. Recovery Unit

Recovery의 최소 작업 단위는 가능한 한 지역적이어야 한다.

예:

```text
single table
single figure + caption
single page region
equation block
multi-column region
```

전체 문서 recovery는 기본 경로가 아니다.

# 14. R6 — Recovery Adapter

초기 지능 복구는 외부 multimodal model을 사용할 수 있다.

Adapter 인터페이스는 공급자와 분리한다.

```text
RecoveryRequest
RecoveryResult
```

## RecoveryRequest

```text
region image / source representation
nearby deterministic text
suspected structure
failure reason
required output schema
```

## RecoveryResult

```text
recovered nodes
recovered relations
uncertainty
source references
```

특정 Gemini 모델명은 이 인터페이스의 계약이 아니다.

# 15. Recovery 결과 권위

Recovery 결과는 다음 상태에서 시작한다.

```text
UNVERIFIED_RECOVERY
```

바로 Canonical Source에 기록하지 않는다.

다음 과정을 통과해야 한다.

```text
Recovery
→ Source Rebinding
→ Structural Validation
→ Consistency Validation
→ ACCEPT / PARTIAL / REJECT
```

# 16. Re-validation

복구 결과에 최소한 다음을 검사한다.

- 복구된 문자열이 실제 source region과 일치하는가
- 셀/행/열 구조가 좌표와 모순되지 않는가
- 새 content가 원문 없이 생성되지 않았는가
- deterministic extraction과 충돌하는가
- target region 밖 content가 섞이지 않았는가

결과:

```text
RECOVERY_VERIFIED
RECOVERY_PARTIAL
RECOVERY_REJECTED
```

# 17. Canonicalizer

검증된 Raw/Recovered Source를 공통 Canonical Source로 변환한다.

Canonicalizer는 의미를 추가하지 않는다.

대표 node:

```text
Document
Section
Paragraph
List
Table
Row
Cell
Caption
Footnote
FigureRegion
EquationRegion
```

# 18. Canonical Node 최소 구조

```text
CanonicalNode {
    source_node_id
    node_type
    content
    parent_id
    document_order
    source_location
    extraction_origin
    extraction_status
}
```

`extraction_origin` 예:

```text
DOCX_DETERMINISTIC
PDF_DETERMINISTIC
RECOVERY_VERIFIED
```

# 19. Stable Anchor Builder

Anchor는 Canonical Node와 원문을 연결한다.

최소 구조:

```text
StableAnchor {
    anchor_id
    document_revision_id
    source_node_id
    local_start
    local_end
    quote
    quote_hash
    source_location
}
```

구조형 node에는 선택적으로:

```text
page
bounding_region
table_id
row
column
figure_id
footnote_id
```

를 포함한다.

# 20. R8 — Source Resolver

다음 연산을 지원한다.

```text
resolve(anchor)
→ source content

resolve_with_context(anchor, before, after)
→ surrounding source

resolve_region(source_node_id)
→ original region
```

후속 시스템은 Parser 내부의 chunk 또는 temporary ID를 알 필요가 없다.

# 21. Document Revision

Parser output은 원본과 parser configuration에 결속한다.

최소 개념:

```text
document_revision_id
original_hash
format
parser_schema_version
extractor_profile
created_at
```

동일 원본이라도 Canonical representation이 material하게 달라지면 새 revision으로 취급할 수 있다.

# 22. 최소 상태 모델

문서 전체 처리 상태:

```text
RECEIVED
EXTRACTED
VALIDATED
RECOVERY_REQUIRED
RECOVERED
CANONICALIZED
READY

FAILED
PARTIAL
```

기본 전이:

```text
RECEIVED
→ EXTRACTED
→ VALIDATED
→ CANONICALIZED
→ READY
```

Recovery 경로:

```text
VALIDATED
→ RECOVERY_REQUIRED
→ RECOVERED
→ VALIDATED
→ CANONICALIZED
```

# 23. 실패 상태

최소 오류 유형:

```text
INPUT_UNSUPPORTED
INPUT_DAMAGED

EXTRACTION_FAILED
EXTRACTION_PARTIAL

STRUCTURE_AMBIGUOUS
STRUCTURE_INVALID

RECOVERY_FAILED
RECOVERY_REJECTED

SOURCE_UNRESOLVED
```

오류는 내용 부재를 의미하지 않는다.

# 24. 외부 의존성 경계

다음은 교체 가능한 dependency다.

```text
DOCX decoder
PDF text extractor
PDF coordinate extractor
image decoder
OCR engine
multimodal recovery provider
```

상위 코드는 vendor-specific API에 직접 의존하지 않는다.

Adapter boundary를 통해 접근한다.

# 25. 구현 언어

기준 구현은 Rust로 둔다.

Python runtime을 parser core의 필수 dependency로 만들지 않는다.

단, 외부 recovery API는 네트워크 경계로 사용 가능하다.

# 26. 초기 crate/module 구조

```text
src/
├─ lib.rs
├─ input/
│  └─ inspector.rs
├─ adapters/
│  ├─ mod.rs
│  ├─ docx.rs
│  └─ pdf.rs
├─ raw/
│  └─ model.rs
├─ integrity/
│  ├─ mod.rs
│  ├─ reading_order.rs
│  ├─ table.rs
│  ├─ caption.rs
│  └─ footnote.rs
├─ recovery/
│  ├─ mod.rs
│  ├─ router.rs
│  └─ provider.rs
├─ canonical/
│  ├─ model.rs
│  └─ canonicalizer.rs
├─ anchor/
│  ├─ model.rs
│  └─ resolver.rs
└─ error.rs
```

이 구조는 책임 분리를 위한 초기 형태이며 파일 수 자체는 불변이 아니다.

# 27. Persistence

Parser core는 기본적으로 stateless하게 설계한다.

```text
input
→ output
```

Canonical Source의 영속 저장은 소비 시스템의 책임으로 둘 수 있다.

Parser 자체가 DB를 필수로 요구하지 않는다.

# 28. Concurrency

초기 MVP는 복잡한 concurrency architecture를 요구하지 않는다.

각 문서 parsing run은 독립적이어야 한다.

공유 mutable state를 최소화한다.

외부 recovery 호출은 별도 request 단위로 격리한다.

# 29. 성능 경계

정상 deterministic path:

```text
network model call = 0
```

측정 대상:

```text
parse latency
validation latency
peak RSS
output size
recovery invocation rate
recovery latency
recovery cost
```

구체 수치 목표는 별도 성능 계약에서 고정한다.

# 30. 보안 및 데이터 경계

기본 deterministic extraction은 로컬 실행이 가능해야 한다.

외부 recovery가 필요한 경우에만 필요한 최소 영역을 전송한다.

```text
whole document
→ 기본 전송 금지

affected region
→ 필요한 경우 허용
```

Recovery provider가 필요로 하지 않는 문맥을 추가 전송하지 않는다.

# 31. 테스트 계층

## Layer 1 — Unit

- canonical model
- anchors
- integrity rules
- state transitions
- error classification

## Layer 2 — Adapter

- DOCX extraction
- PDF primitives
- source positions

## Layer 3 — Structural Fixtures

- merged table
- multi-level header
- multicolumn PDF
- caption
- footnote
- equation region

## Layer 4 — Recovery

```text
structural failure
→ detection
→ targeted recovery
→ re-validation
```

## Layer 5 — End-to-End

```text
document
→ READY
→ Canonical Source
→ Stable Anchor
→ resolve original source
```

# 32. 핵심 수용조건

## AC-01 DOCX

구조화 DOCX에서 section/paragraph/table/cell/caption/footnote가 원본 관계를 유지한다.

## AC-02 PDF

Digital PDF에서 text와 source location이 보존된다.

## AC-03 Reading Order

다단 또는 복잡 layout에서 명백한 ordering anomaly를 정상 Source로 조용히 승격하지 않는다.

## AC-04 Table

병합/다단 header fixture에서 구조 손실을 탐지하거나 정상 구조를 복원한다.

## AC-05 Targeted Recovery

정상 문서는 recovery provider를 호출하지 않는다.

## AC-06 Recovery Scope

문제 영역만 recovery request에 포함한다.

## AC-07 Recovery Authority

Recovery 결과를 재검증 없이 Canonical Source로 승격하지 않는다.

## AC-08 Source Binding

Canonical material node에서 Stable Anchor를 통해 원문을 다시 확인할 수 있다.

## AC-09 Failure Separation

추출·복구 실패를 semantic absence로 반환하지 않는다.

## AC-10 Replaceability

DOCX/PDF extractor 또는 recovery provider 교체가 Canonical Source consumer의 변경을 요구하지 않는다.

# 33. 구현 순서

```text
1. Canonical Source + Stable Anchor
2. DOCX deterministic path
3. PDF text/position path
4. Structure Integrity Validator
5. Recovery Router
6. Recovery Adapter
7. Recovery Re-validation
8. Full acceptance fixtures
```

# 34. 첫 최소 세로절단

첫 구현 결과:

```text
DOCX fixture
→ deterministic extraction
→ structural validation
→ Canonical Source
→ Stable Anchor
→ source resolve
```

이 단계에서는 Gemini나 PDF까지 동시에 구현할 필요가 없다.

# 35. 구현 금지

초기 구현에서 다음을 하지 않는다.

- 자체 PDF 렌더링 엔진 제작
- 자체 OCR 모델 제작
- 전체 문서 LLM parsing
- 의미 판단
- 범용 ontology
- Agent orchestration
- domain-specific semantics
- 불필요한 DB
- speculative plugin framework
- 미래 문서 형식 선행 지원

# 36. 구현 완료 판정

Parser Core MVP 완료는 다음 경로가 실제 fixture에서 통과할 때 선언한다.

```text
DOCX / Digital PDF
→ deterministic extraction
→ integrity validation
→ targeted recovery only when required
→ re-validation
→ Canonical Source
→ Stable Anchor
→ original source resolution
```

그리고:

- 정상 문서는 모델 호출 0
- 이상 구조는 조용히 정상 처리되지 않음
- recovery 결과는 검증 후에만 채택
- Source 위치 재확인 가능
- extraction failure ≠ absence
- adapter 교체 가능

이 모두 관찰 가능하게 검증돼야 한다.
