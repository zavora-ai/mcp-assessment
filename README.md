# Assessment MCP Server

[![Crates.io](https://img.shields.io/crates/v/mcp-assessment.svg)](https://crates.io/crates/mcp-assessment)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)
[![Registry Ready](https://img.shields.io/badge/ADK_Registry-Ready-green.svg)](https://www.zavora.ai)

An assessment platform for [ADK-Rust Enterprise](https://enterprise.adk-rust.com) education agents. 23 MCP tools covering an **item bank** (six item types with keyed answers), **standards/curriculum mapping**, **assessment authoring + blueprint generation**, **rubric scoring**, **attempts with auto-grading**, and **psychometric analytics** (item difficulty/discrimination and standard mastery) — with a full audit trail.

## A platform, not a point solution

This is modeled as a general assessment engine (à la a test/item-banking system behind an LMS), so the education agents are simply clients:

| Agent | Uses |
|-------|------|
| **Quiz Generator** | `generate_assessment` (difficulty blueprint over the item bank), `create_item` |
| **Curriculum Mapping Agent** | `add_standard`, `list_standards`, `items_for_standard`, `assessment_blueprint` |
| **Instructor Copilot** | `assessment_blueprint`, `item_analysis`, `list_attempts`, `set_assessment_status` |
| **Rubric Grading Assistant** | `create_rubric`, `score_rubric_item` |
| **Adaptive Tutor Agent** | `standard_mastery`, `learner_attempts`, `items_for_standard` |

## Architecture

<p align="center">
  <img src="https://raw.githubusercontent.com/zavora-ai/mcp-assessment/main/docs/architecture.svg" alt="Assessment MCP Architecture" width="780"/>
</p>

## Capabilities

- **Item bank** — six item types: `multiple_choice`, `multiple_select`, `true_false`, `short_answer`, `numeric` (with tolerance), and `essay` (rubric-scored). Items carry difficulty, Bloom's cognitive level, point value, and **standard alignment**. Authoring is validated per type.
- **Standards / curriculum mapping** — learning standards (CCSS/NGSS/custom) with `items_for_standard` coverage and per-assessment standard breakdowns.
- **Authoring & generation** — build assessments from explicit items, or **generate from a blueprint** (difficulty → count, optionally constrained to standards) drawing from the bank.
- **Attempts & grading** — learner attempts auto-grade objective items on submit; **rubric/essay items** are scored per-criterion (clamped to each criterion's max) and the attempt finalizes with score, percent, and pass/fail.
- **Psychometrics** — `item_analysis` computes per-item **p-value** (difficulty) and **discrimination** (top-third vs bottom-third correct-rate) with quality flags (too_hard / too_easy / low_discrimination). `standard_mastery` rolls a learner's earned points per standard into proficiency levels.

## Governance posture

- **Two writes are gated** (`requires_approval`): publishing an assessment (`set_assessment_status` → published — makes it live for learners) and **submitting an attempt** (`submit_attempt`, classed `external_write` — it records a graded result of record).
- **Publishing is guarded** — an assessment can't be published with no items, and only published assessments accept attempts.
- **All analytics and bank queries are `read_only`**.
- **Everything is audited** (`audit_log`). Sample data is fictitious.

## Tools (23)

### Standards & Curriculum (3)
`add_standard` · `list_standards` · `items_for_standard`

### Rubrics (2)
`create_rubric` · `get_rubric`

### Item Bank (3)
`create_item` · `get_item` · `list_items`

### Assessments (6)
`create_assessment` · `generate_assessment` · `get_assessment` · `list_assessments` · `set_assessment_status` (gated) · `assessment_blueprint`

### Attempts & Grading (6)
`start_attempt` · `submit_attempt` (gated, external) · `score_rubric_item` · `get_attempt` · `list_attempts` · `learner_attempts`

### Analytics (3)
`item_analysis` · `standard_mastery` · `audit_log`

## Example

```jsonc
// Quiz Generator: build from a difficulty blueprint
{"name": "generate_assessment", "arguments": {"title": "Algebra Quiz", "subject": "Mathematics",
  "blueprint": [{"difficulty": "easy", "count": 3}, {"difficulty": "medium", "count": 2}]}}
{"name": "set_assessment_status", "arguments": {"assessment_id": "ASM-1012", "status": "published"}}

// Attempt + auto-grade
{"name": "start_attempt", "arguments": {"assessment_id": "ASM-1012", "learner_id": "L1"}}
{"name": "submit_attempt", "arguments": {"attempt_id": "ATT-1020",
  "responses": [{"item_id": "ITM-1001", "value": 4}]}}

// Instructor Copilot + Adaptive Tutor
{"name": "item_analysis", "arguments": {"assessment_id": "ASM-1012"}}
{"name": "standard_mastery", "arguments": {"learner_id": "L1"}}
```

## Install & run

```bash
cargo install mcp-assessment
mcp-assessment            # serves MCP over stdio
```

Or build from source:

```bash
git clone https://github.com/zavora-ai/mcp-assessment
cd mcp-assessment && cargo build --release
./target/release/mcp-assessment
```

## Registry manifest

```toml
server_id = "mcp_assessment"
display_name = "Assessment Platform"
version = "1.0.0"
domain = "education"
risk_level = "medium"
writes_allowed = "gated"
```

The full [`mcp-server.toml`](mcp-server.toml) declares all 23 tools with risk classes and approval gates for registry onboarding.

## License

Apache-2.0

## rmcp and MCP compatibility

This server is built with [`rmcp` 3.1.2](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v3.1.2) and requires Rust 1.88 or newer. The rmcp 3 rollout retains legacy MCP initialization compatibility and targets MCP protocol revisions `2025-11-25` and `2026-07-28`.
