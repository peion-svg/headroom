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

# 33. 8단계 세로절단

구현은 다음 8단계로 확장한다. 각 단계는 앞 단계의 검증된 결과 위에 하나의 책임을 추가하며, 이후 단계 기능을 선행 구현하지 않는다.

공통 유의사항:

- 외부 parser/library의 동작을 설계 권위로 승격하지 않는다.
- 구현 중 발견된 문제 때문에 임의로 다음 Stage 책임이나 WorkContract 범위를 끌어오지 않는다.
- 기술적 실패를 내용 부재나 정상 Source로 바꾸지 않는다.
- 성능·메모리·정확도 문제는 해당 adapter 또는 Stage에서 먼저 격리한다.
- 구현 세부 변경이 Canonical Source, Stable Anchor, Source Binding 불변을 깨면 즉시 HOLD한다.

## Stage 1 — Canonical Source + Stable Anchor Foundation

```text
validated source object
→ Canonical Source
→ Stable Anchor
→ source resolve
```

공통 Source identity, anchor, resolver의 최소 계약을 고정한다.

### 구현 유의사항

- 특정 DOCX/PDF library의 transient ID를 `source_node_id`나 anchor identity로 사용하지 않는다.
- 미래 포맷까지 예상한 범용 framework를 먼저 만들지 않는다.
- 동일 source가 재실행·재조회에서 다른 객체로 오인되지 않도록 identity 생성 규칙을 deterministic하게 유지한다.
- anchor가 생성되었다는 사실만으로 충분하지 않다. 실제 원문으로 roundtrip resolve되어야 한다.
- source 위치를 잃은 node를 정상 material source처럼 승격하지 않는다.
- Table/Row처럼 구조 표현을 위해 조합한 synthetic text를 원문 quote/content와 동일시하지 않는다.
- Stable Anchor의 offset 단위는 구현에서 하나로 고정하고 byte / Unicode scalar / UTF-16을 혼용하지 않는다.
- 동일 문구가 여러 위치에 존재할 수 있으므로 quote/hash만으로 source 위치를 재결정하지 않는다. source_node_id와 source_location 결속을 우선한다.
- document revision은 최소 원본 hash + parser schema + extractor profile에 결속하고, material representation이 달라지는 extractor 변경을 동일 revision으로 숨기지 않는다.

## Stage 2 — DOCX Deterministic Path

```text
DOCX
→ deterministic extraction
→ Canonical Source
→ Stable Anchor
→ source resolve
```

DOCX 구조를 deterministic하게 추출하고 Stage 1 결과에 연결한다.

### 구현 유의사항

- DOCX XML에 존재하는 구조를 의미적으로 재작성하거나 추측하지 않는다.
- heading, list, table, merged cell, caption, footnote 관계가 라이브러리 wrapper에서 유실되는지 fixture로 확인한다.
- library가 편의를 위해 평탄화한 결과를 원본 구조와 동일하다고 가정하지 않는다.
- malformed/unsupported 입력을 빈 문서로 반환하지 않는다.
- 특정 fixture만 통과시키기 위한 hard-coded 구조를 만들지 않는다.
- DOCX 처리 문제 때문에 PDF/recovery 책임을 끌어오지 않는다.
- heading 뒤에 있다는 이유만으로 후속 paragraph/table을 heading의 자식으로 만드는 등 원문에 없는 containment를 추론하지 않는다.
- DOCX table의 physical cell vector index와 logical grid column을 동일시하지 않는다. gridSpan, gridBefore/gridAfter, vertical merge가 있는 경우 logical coordinate를 별도로 검증한다.
- list 여부와 level이 direct paragraph property만이 아니라 numbering/style inheritance에서 결정될 수 있음을 고려한다.
- tracked changes, bookmarks, fields, hyperlinks, content controls 등 common-but-unmodeled XML을 무조건 정상 구조 또는 무조건 실패로 뭉개지 말고 보존 가능 / 구조 영향 / unsupported를 구분한다.
- text가 비어 있다는 이유만으로 paragraph를 즉시 버리지 않는다. section break, spacing, 경계 등 구조 정보가 남아 있는지 확인한다.
- Caption style의 존재와 caption-target 관계 확정은 별개다. target 관계가 명시되지 않았다면 임의 결속하지 않는다.
- footnote/endnote/comment처럼 body 밖 별도 story의 위치 namespace를 본문과 혼합하지 않는다.
- encrypted, damaged package, malformed XML, unsupported structure를 하나의 generic extraction failure로 뭉개지 않는다.

## Stage 3 — Digital PDF Text/Position Path

```text
Digital PDF
→ text + position primitives
→ Canonical Source
→ Stable Anchor
→ source resolve
```

PDF 의미 구조를 과잉 추론하지 않고 text/position 기반 Source를 보존한다.

### 구현 유의사항

- PDF text extraction 성공을 reading order 성공으로 간주하지 않는다.
- page, bounding box, span order 등 원위치 정보를 가능한 한 보존한다.
- 다단 문서, header/footer, 표 내부 text가 평탄화될 수 있음을 전제로 한다.
- extractor가 반환한 순서를 곧바로 논리 문서 순서로 확정하지 않는다.
- 추출 text가 적거나 비어 있다고 실제 문서 내용이 없다고 판정하지 않는다.
- 특정 PDF library의 결과 차이는 adapter 문제로 격리하고 Canonical 계약을 변경하는 이유로 사용하지 않는다.
- 동일 text가 반복되는 페이지/영역에서 text match만으로 bounding region을 재결정하지 않는다.
- page rotation, crop/media box, coordinate origin 차이 등 좌표계 변환으로 bbox가 드리프트하지 않는지 확인한다.
- extractor가 일부 glyph/span을 조용히 누락하거나 중복해도 API 성공을 extraction completeness로 간주하지 않는다.

## Stage 4 — Structure Integrity Validator

```text
extracted structure
→ integrity rules
→ VALID / AMBIGUOUS / PARTIAL / FAILED
→ reason + evidence + affected region
```

구조 이상을 조용히 정상 Source로 승격하지 않게 한다.

### 구현 유의사항

- 가장 위험한 실패는 오류를 내는 것이 아니라 틀린 구조를 `VALID`로 통과시키는 것이다.
- validator는 의미적 정답을 판단하지 않고 관찰 가능한 구조 모순만 판정한다.
- 불확실성을 억지로 PASS/FAIL 이진값으로 압축하지 말고 `AMBIGUOUS`와 `PARTIAL`을 사용한다.
- 각 판정에는 reason code, evidence, affected region을 남긴다.
- validator rule이 특정 문서군에 과적합되어 정상 문서를 대량 recovery로 보내지 않는지 확인한다.
- false negative와 false positive를 모두 fixture로 관리한다.
- 외부 library가 오류 없이 구조 일부를 조용히 버리는 silent-loss를 별도 이상 유형으로 검출할 수 있어야 한다.
- 한 region의 PARTIAL/AMBIGUOUS가 문서 전체를 무조건 FAILED로 만들거나 반대로 전체 VALID로 덮이지 않도록 failure scope를 보존한다.
- table logical coordinates와 physical cell indices가 어긋나는 경우를 integrity evidence로 검증한다.
- common-but-unmodeled XML이 존재한다는 사실 자체와 실제 material structure loss를 구분한다.

## Stage 5 — Recovery Router

```text
integrity finding
→ recovery decision
→ NO_RECOVERY / RECOVERY_REQUIRED / RECOVERY_UNAVAILABLE
```

모델 호출 여부와 대상 영역을 결정한다. 전체 문서를 기본 recovery 단위로 사용하지 않는다.

### 구현 유의사항

- 모델이 사용 가능하다는 이유만으로 호출하지 않는다.
- `VALID` 영역을 편의상 recovery에 포함하지 않는다.
- affected region을 필요 이상으로 넓혀 전체 문서 recovery로 퇴행하지 않는다.
- 비용 절감 때문에 필요한 recovery를 생략하거나, 품질 불안 때문에 모든 영역을 recovery하지 않는다.
- provider 장애와 문서 구조 실패를 동일 실패로 취급하지 않는다.
- recovery 불가 시 authority를 확대하지 말고 `RECOVERY_UNAVAILABLE` 또는 HOLD로 남긴다.

## Stage 6 — Targeted Recovery Adapter

```text
affected region
→ targeted recovery request
→ UNVERIFIED_RECOVERY
```

교체 가능한 recovery provider를 통해 문제 영역만 복구한다.

### 구현 유의사항

- 모델 출력은 항상 `UNVERIFIED_RECOVERY`로 시작한다.
- 원문에 없는 text, cell, 관계, 수치를 생성해도 그럴듯하다는 이유로 채택하지 않는다.
- provider-specific schema와 ID를 상위 Canonical 계약으로 누출하지 않는다.
- 요청에는 복구에 필요한 최소 원문 영역과 주변 deterministic context만 포함한다.
- 모델 버전·응답 형식 변화가 parser 전체 identity를 흔들지 않도록 adapter에서 격리한다.
- timeout, malformed response, partial response를 명시적으로 실패 상태로 반환한다.

## Stage 7 — Recovery Re-validation + Source Rebinding

```text
UNVERIFIED_RECOVERY
→ source rebinding
→ structural + consistency validation
→ VERIFIED / PARTIAL / REJECTED
→ Canonical Source when verified
```

검증되지 않은 모델 출력을 Canonical Source로 승격하지 않는다.

### 구현 유의사항

- 이 단계는 Stage 4와 함께 가장 높은 위험 구간이다.
- 복구 결과가 원본 region에 실제로 다시 결속되는지 확인하지 못하면 `VERIFIED`가 아니다.
- 모델 출력과 deterministic extraction이 충돌하면 모델을 자동 우선하지 않는다.
- quote/text match만 보고 구조 관계까지 검증됐다고 간주하지 않는다.
- 부분적으로만 확인 가능한 결과는 `PARTIAL`로 남기고 정상 Source로 확장 해석하지 않는다.
- rebinding 실패를 content absence로 바꾸지 않는다.
- recovery 성공률을 높이기 위해 검증 기준을 완화하지 않는다.
- 내용이 맞아 보여도 recovered structure의 좌표/bounding region이 원본과 어긋나면 VERIFIED로 승격하지 않는다.
- content match와 position match를 독립적으로 검증한다.
- 동일 문구 중복으로 잘못된 region에 rebinding되는 경우를 거부한다.
- 부분적으로 검증된 region의 성공을 인접 실패 region까지 확장하지 않는다.

## Stage 8 — Full Acceptance Closure

```text
DOCX / Digital PDF / failure fixture / recovery fixture
→ full pipeline
→ acceptance criteria
→ PASS / HOLD / FAIL
```

전체 MVP 경로와 회귀 fixture를 검증하여 구현 완료 여부를 판정한다.

### 구현 유의사항

- happy path만으로 PASS하지 않는다.
- malformed, ambiguous, partial, provider failure, rebinding failure fixture를 포함한다.
- 정상 deterministic 문서에서 모델 호출 0인지 확인한다.
- parser/library 버전 변경 후 동일 fixture의 material output drift를 확인한다.
- 반복 처리·다문서 처리에서 상태 오염이나 결과 혼선이 없는지 확인한다.
- 장시간 또는 반복 실행 시 메모리 증가가 지속되는지 측정하되, 원인 미확정 상태에서 leak로 단정하지 않는다.
- 수용조건 미충족을 문서 해석이나 수동 승인으로 덮지 않는다.
- Stage 8 PASS는 현재 MVP 구현 완료만 의미하며 제품 의미판단 또는 후속 SCI 검수의 정확성을 보증하지 않는다.
- 같은 bytes + 같은 parser schema + 같은 extractor profile에서 node/anchor identity뿐 아니라 material Canonical output이 deterministic한지 회귀 검증한다.
- extractor/library 버전 변경 전후의 snapshot diff에서 material drift와 비material drift를 구분한다.
- 압축폭탄, 과대 XML/table/image, 과도한 중첩 등 resource-exhaustion 입력에 명시적 한도와 failure가 있는지 확인한다.
- 필수 build/test gate를 실제로 실행할 환경이 없으면 정적 검수로 대체해 PASS하지 않고 HOLD한다.

## WorkContract mapping

8단계는 구현 책임 순서다. WorkContract는 실행 가능한 최소 결과를 만들기 위해 인접 단계를 묶을 수 있다.

현재 `WC-PARSER-001`은 Stage 1과 Stage 2를 묶은 첫 실행 가능한 세로절단이다.

```text
DOCX fixture
→ deterministic extraction
→ structural validation
→ Canonical Source
→ Stable Anchor
→ source resolve
```

이 계약에서는 PDF, recovery, OCR 또는 이후 Stage를 구현하지 않는다.

## 8단계 구현 체크리스트

### Stage 1 — Canonical Source + Stable Anchor Foundation
- [ ] Source identity가 외부 library transient ID와 분리되어 있는가
- [ ] Canonical node 최소 필드가 실제 구현에 존재하는가
- [ ] Stable Anchor가 원문 위치를 보존하는가
- [ ] anchor → source roundtrip resolve가 가능한가
- [ ] 위치를 잃은 material node가 정상 승격되지 않는가
- [ ] 미래 포맷용 speculative abstraction을 추가하지 않았는가
- [ ] direct source text와 synthetic structural text가 구분되는가
- [ ] Stable Anchor offset 단위가 하나로 고정되어 있는가
- [ ] 동일 quote가 반복돼도 source_node_id + source_location으로 위치가 고정되는가
- [ ] document revision이 원본 hash + parser schema + extractor profile에 결속되는가

### Stage 2 — DOCX Deterministic Path
- [ ] heading/paragraph/list/table 관계가 보존되는가
- [ ] merged cell 또는 multi-level header 관계가 보존되는가
- [ ] caption/footnote 중 fixture에 존재하는 구조가 보존되는가
- [ ] document order가 유지되는가
- [ ] malformed/unsupported DOCX가 명시적 실패를 반환하는가
- [ ] 빈 추출 결과를 content absence로 처리하지 않는가
- [ ] PDF/recovery 책임을 선행 구현하지 않았는가
- [ ] 원문에 없는 heading containment 등 계층을 추론하지 않는가
- [ ] physical cell index와 logical table column을 분리해 검증하는가
- [ ] list inheritance를 direct property만으로 오판하지 않는가
- [ ] common-but-unmodeled XML을 보존 가능/구조 영향/unsupported로 구분하는가
- [ ] empty paragraph의 구조 정보를 확인하지 않고 삭제하지 않는가
- [ ] caption 존재와 caption-target 관계를 별도로 취급하는가
- [ ] footnote/endnote/comment story namespace가 body와 분리되는가
- [ ] encrypted/damaged/malformed/unsupported failure가 구분되는가

### Stage 3 — Digital PDF Text/Position Path
- [ ] page/text span/bounding position이 보존되는가
- [ ] extractor 순서를 논리 reading order로 자동 확정하지 않는가
- [ ] multi-column/header/footer/table text 평탄화 위험을 기록하는가
- [ ] text 부족 또는 공백을 content absence로 처리하지 않는가
- [ ] PDF library 교체가 Canonical 계약 변경을 요구하지 않는가
- [ ] 반복 text에서 text match만으로 bbox를 재결정하지 않는가
- [ ] PDF 좌표계 변환 후 bounding box drift를 검증하는가
- [ ] API 성공을 extraction completeness로 간주하지 않는가

### Stage 4 — Structure Integrity Validator
- [ ] VALID/AMBIGUOUS/PARTIAL/FAILED를 구분하는가
- [ ] 각 판정에 reason code가 남는가
- [ ] evidence와 affected region이 남는가
- [ ] 의미판단이 아니라 관찰 가능한 구조 모순만 검사하는가
- [ ] false negative fixture가 있는가
- [ ] false positive fixture가 있는가
- [ ] 틀린 구조가 조용히 VALID로 통과하지 않는가
- [ ] silent structure loss를 탐지하는 fixture가 있는가
- [ ] region-level PARTIAL/AMBIGUOUS scope가 문서 전체 판정에 왜곡되지 않는가
- [ ] logical table coordinate 불일치를 검출하는가
- [ ] unmodeled XML 존재와 material structure loss를 구분하는가

### Stage 5 — Recovery Router
- [ ] VALID 영역에는 recovery를 호출하지 않는가
- [ ] affected region만 recovery 대상으로 선택하는가
- [ ] 전체 문서 recovery가 기본 경로가 아닌가
- [ ] NO_RECOVERY / RECOVERY_REQUIRED / RECOVERY_UNAVAILABLE가 구분되는가
- [ ] provider 장애와 document structure failure가 구분되는가
- [ ] recovery 불가 시 authority를 임의 확대하지 않는가

### Stage 6 — Targeted Recovery Adapter
- [ ] 모든 모델 출력이 UNVERIFIED_RECOVERY로 시작하는가
- [ ] 요청 범위가 필요한 최소 source region으로 제한되는가
- [ ] provider-specific ID/schema가 Canonical 계층으로 누출되지 않는가
- [ ] timeout/malformed/partial response가 명시적 상태로 반환되는가
- [ ] 원문에 없는 text/value/relation을 정상 결과로 창작하지 않는가
- [ ] provider/model 버전 변경이 adapter 내부에 격리되는가

### Stage 7 — Recovery Re-validation + Source Rebinding
- [ ] recovered content가 실제 원문 region에 다시 결속되는가
- [ ] 구조 관계까지 재검증하는가
- [ ] deterministic extraction과 충돌 시 모델을 자동 우선하지 않는가
- [ ] VERIFIED/PARTIAL/REJECTED가 구분되는가
- [ ] PARTIAL을 완전한 Canonical Source로 확장하지 않는가
- [ ] rebinding 실패를 content absence로 바꾸지 않는가
- [ ] 검증 기준을 recovery 성공률 때문에 완화하지 않는가
- [ ] recovered content와 recovered position을 독립적으로 검증하는가
- [ ] 동일 문구 중복으로 잘못된 region에 rebinding되지 않는가
- [ ] PARTIAL 성공을 인접 실패 영역으로 확장하지 않는가

### Stage 8 — Full Acceptance Closure
- [ ] DOCX happy-path fixture가 통과하는가
- [ ] Digital PDF fixture가 통과하는가
- [ ] malformed/ambiguous/partial fixture가 포함되는가
- [ ] provider failure/rebinding failure fixture가 포함되는가
- [ ] 정상 deterministic 문서에서 model call = 0인가
- [ ] 반복·다문서 처리에서 상태 오염이 없는가
- [ ] parser/library 버전 변경 시 material output drift를 확인하는가
- [ ] 반복 실행 메모리 변화를 측정하는가
- [ ] 모든 acceptance criteria가 관찰 가능한 증거로 판정되는가
- [ ] same bytes + same parser schema + same extractor profile에서 material output이 deterministic한가
- [ ] library/extractor version drift를 snapshot diff로 확인하는가
- [ ] 과대/비정상 입력에 resource limit과 명시적 failure가 있는가
- [ ] 필수 build/test gate를 실제 실행하지 못한 경우 PASS 대신 HOLD하는가

### 공통 완료 체크
- [ ] 현재 Stage의 Required Result가 실제 fixture에서 재현되는가
- [ ] 해당 Stage의 실패 상태가 명시적으로 표현되는가
- [ ] 현재 WorkContract Boundary 밖 구현이 없는가
- [ ] Canonical Source / Stable Anchor / Source Binding 불변이 유지되는가
- [ ] 테스트하지 않은 항목을 PASS로 기록하지 않는가
- [ ] 다음 Stage로 넘어가기 전에 현재 Stage 상태를 STATUS.md에 반영하는가
- [ ] static review/API 대조를 실제 build/test 실행의 대체 증거로 사용하지 않는가

# 34. 현재 첫 최소 세로절단

현재 활성 구현 결과는 `WC-PARSER-001`의 DOCX 경로다.

Stage 1의 공통 기반은 독립적인 speculative framework로 만들지 않고, DOCX 경로가 실제로 요구하는 최소 형태로 구현한다.

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
