//! Assessment platform domain model.
//!
//! Broad assessment backbone: an item bank with several item types and keyed
//! answers, learning standards (curriculum mapping), rubrics with weighted
//! criteria, assessments composed of items (hand-built or blueprint-generated),
//! attempts with per-item responses, auto-grading, and psychometric analytics
//! (item difficulty/discrimination, standard mastery). The named agents
//! (Adaptive Tutor, Curriculum Mapping, Instructor Copilot, Quiz Generator,
//! Rubric Grading Assistant) are clients of this platform.

use chrono::{DateTime, Utc};
use rmcp::schemars;
use serde::{Deserialize, Serialize};

// ─── items ─────────────────────────────────────────────────────────────────

/// Item (question) types. Objective types auto-grade; constructed types need a
/// rubric or human/AI scoring.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// One correct choice.
    MultipleChoice,
    /// Multiple correct choices (all-or-nothing unless partial credit).
    MultipleSelect,
    TrueFalse,
    /// Short text matched against accepted answers.
    ShortAnswer,
    /// Numeric answer with optional tolerance.
    Numeric,
    /// Free response — rubric-scored.
    Essay,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

/// Bloom's taxonomy cognitive level — useful for blueprints and balance.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Cognitive {
    Remember,
    Understand,
    Apply,
    Analyze,
    Evaluate,
    Create,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Item {
    pub id: String,
    pub stem: String,
    pub itype: ItemType,
    pub subject: String,
    pub difficulty: Difficulty,
    pub cognitive: Cognitive,
    pub points: f64,
    /// Choices for choice-type items (label is the option text).
    pub options: Vec<String>,
    /// Indices (into `options`) or accepted values that count as correct.
    /// For numeric: a single value string; tolerance in `tolerance`.
    pub answer_key: Vec<String>,
    pub tolerance: Option<f64>,
    /// Standards this item assesses.
    pub standard_ids: Vec<String>,
    pub rubric_id: Option<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
}

// ─── standards (curriculum) ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Standard {
    pub id: String,
    /// e.g. "CCSS.MATH.HSA.REI.B.3".
    pub code: String,
    pub framework: String, // CCSS, NGSS, custom...
    pub subject: String,
    pub grade: String,
    pub description: String,
}

// ─── rubrics ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct RubricCriterion {
    pub name: String,
    pub description: String,
    /// Max points for this criterion.
    pub max_points: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Rubric {
    pub id: String,
    pub name: String,
    pub criteria: Vec<RubricCriterion>,
    pub created_at: DateTime<Utc>,
}

// ─── assessments ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Draft,
    Published,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Assessment {
    pub id: String,
    pub title: String,
    pub subject: String,
    pub status: AssessmentStatus,
    /// Ordered item ids.
    pub item_ids: Vec<String>,
    pub total_points: f64,
    /// Fraction (0..1) needed to pass.
    pub pass_threshold: f64,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
}

// ─── attempts & responses ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, schemars::JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    InProgress,
    Submitted,
    Graded,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ItemResponse {
    pub item_id: String,
    /// Learner's answer: string, number, or array (multi-select).
    pub value: serde_json::Value,
    /// Set after grading.
    pub awarded_points: Option<f64>,
    pub correct: Option<bool>,
    /// Per-criterion rubric scores (for rubric-graded items).
    pub rubric_scores: Vec<f64>,
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Attempt {
    pub id: String,
    pub assessment_id: String,
    pub learner_id: String,
    pub status: AttemptStatus,
    pub responses: Vec<ItemResponse>,
    pub score: Option<f64>,
    pub max_score: f64,
    pub percent: Option<f64>,
    pub passed: Option<bool>,
    pub started_at: DateTime<Utc>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub graded_at: Option<DateTime<Utc>>,
}

// ─── audit trail ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AuditEntry {
    pub at: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub detail: String,
}
