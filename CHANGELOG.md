# Changelog

## [1.0.0] - 2026-06-09

Initial release — a broad assessment platform: item bank, authoring/generation, grading, and psychometrics.

### Added
- **Item bank** — 6 item types (multiple_choice, multiple_select, true_false, short_answer, numeric w/ tolerance, essay w/ rubric) with difficulty, Bloom's cognitive level, points, and standard alignment; per-type authoring validation
  (`create_item`, `get_item`, `list_items`)
- **Standards / curriculum mapping** — learning standards with coverage queries
  (`add_standard`, `list_standards`, `items_for_standard`)
- **Rubrics** — weighted criteria for constructed-response scoring
  (`create_rubric`, `get_rubric`)
- **Assessments** — hand-built or **blueprint-generated** (difficulty → count, optional standard constraint); publish gating; blueprint summaries
  (`create_assessment`, `generate_assessment`, `get_assessment`, `list_assessments`, `set_assessment_status`, `assessment_blueprint`)
- **Attempts & grading** — auto-grading across objective types; per-criterion rubric scoring (clamped); score/percent/pass-fail finalization
  (`start_attempt`, `submit_attempt`, `score_rubric_item`, `get_attempt`, `list_attempts`, `learner_attempts`)
- **Psychometrics** — item p-value (difficulty) + discrimination (top vs bottom third) with quality flags; per-standard learner mastery
  (`item_analysis`, `standard_mastery`, `audit_log`)
- 23 tools total; publishing and `submit_attempt` (external write) are gated; full audit trail.
- 13 tests (9 integration + 4 manifest); verified end-to-end over MCP stdio.
