# STATUS

## Repository State

- Project: Lightweight Source Parser
- Repository: `peion-svg/headroom`
- Current phase: implementation preparation
- Implementation status: NOT_STARTED
- Active WorkContract: `WC-PARSER-001`

## Canonical Project Documents

- `docs/DESIGN_BASELINE.md`
- `docs/IMPLEMENTATION_ARCHITECTURE.md`
- `docs/workcontracts/WC-PARSER-001.md`

## 8-Stage Vertical Slice Roadmap

| Stage | Slice | Status |
| --- | --- | --- |
| 1 | Canonical Source + Stable Anchor Foundation | NOT_STARTED |
| 2 | DOCX Deterministic Path | NOT_STARTED |
| 3 | Digital PDF Text/Position Path | NOT_STARTED |
| 4 | Structure Integrity Validator | NOT_STARTED |
| 5 | Recovery Router | NOT_STARTED |
| 6 | Targeted Recovery Adapter | NOT_STARTED |
| 7 | Recovery Re-validation + Source Rebinding | NOT_STARTED |
| 8 | Full Acceptance Closure | NOT_STARTED |

WorkContracts may combine adjacent stages when that is necessary to produce one executable result.

`WC-PARSER-001` covers Stage 1 + Stage 2 as the first executable vertical slice.

## Current Target

```text
DOCX
→ deterministic extraction
→ structural validation
→ Canonical Source
→ Stable Anchor
→ source resolve
```

## Verified So Far

- Repository reset for reuse
- README reduced to stable project information
- Design baseline created
- Implementation architecture created
- 8-stage vertical slice roadmap specified
- Agent work rules created
- First WorkContract created

## Not Yet Verified

- Rust project build
- DOCX extraction
- Canonical Source implementation
- Stable Anchor implementation
- source resolve roundtrip
- fixture-based acceptance tests

## Next Action

Implement `WC-PARSER-001` only.

Do not start Stage 3 or later work before `WC-PARSER-001` reaches PASS or HOLD.
