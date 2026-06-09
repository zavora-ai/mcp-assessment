//! MCP tool surface for the assessment platform.
//!
//! Reads (item bank, blueprints, item analysis, mastery) are `read_only`. Most
//! writes are `internal_write`. Two have real-world weight and are gated
//! (`requires_approval`): publishing an assessment (makes it live for learners)
//! and submitting an attempt (records a graded result of record).

use crate::store::AssessmentStore;
use crate::types::*;
use adk_mcp_sdk::{HealthCheck, HealthStatus};
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde::Deserialize;
use std::sync::Arc;

fn dactor() -> String { "agent".into() }
fn dmath() -> String { "Mathematics".into() }

// ─── inputs ───────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AddStandardInput { pub code: String, #[serde(default = "dccss")] pub framework: String, #[serde(default = "dmath")] pub subject: String, #[serde(default)] pub grade: String, #[serde(default)] pub description: String, #[serde(default = "dactor")] pub actor: String }
fn dccss() -> String { "CCSS".into() }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListStandardsInput { pub subject: Option<String>, pub framework: Option<String> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StandardIdInput { pub standard_id: String }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CriterionInput { pub name: String, #[serde(default)] pub description: String, pub max_points: f64 }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateRubricInput { pub name: String, pub criteria: Vec<CriterionInput>, #[serde(default = "dactor")] pub actor: String }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateItemInput {
    pub stem: String,
    pub itype: ItemType,
    #[serde(default = "dmath")] pub subject: String,
    #[serde(default = "dmed")] pub difficulty: Difficulty,
    #[serde(default = "dapply")] pub cognitive: Cognitive,
    #[serde(default = "done")] pub points: f64,
    #[serde(default)] pub options: Vec<String>,
    #[serde(default)] pub answer_key: Vec<String>,
    pub tolerance: Option<f64>,
    #[serde(default)] pub standard_ids: Vec<String>,
    pub rubric_id: Option<String>,
    #[serde(default = "dactor")] pub actor: String,
}
fn dmed() -> Difficulty { Difficulty::Medium }
fn dapply() -> Cognitive { Cognitive::Apply }
fn done() -> f64 { 1.0 }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ItemIdInput { pub item_id: String }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListItemsInput { pub subject: Option<String>, pub difficulty: Option<Difficulty>, pub standard_id: Option<String> }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateAssessmentInput { pub title: String, #[serde(default = "dmath")] pub subject: String, pub item_ids: Vec<String>, #[serde(default = "dpass")] pub pass_threshold: f64, #[serde(default = "dactor")] pub actor: String }
fn dpass() -> f64 { 0.6 }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BlueprintSlotInput { pub difficulty: Difficulty, pub count: usize }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateAssessmentInput {
    pub title: String,
    #[serde(default = "dmath")] pub subject: String,
    pub blueprint: Vec<BlueprintSlotInput>,
    #[serde(default)] pub standard_ids: Vec<String>,
    #[serde(default = "dpass")] pub pass_threshold: f64,
    #[serde(default = "dactor")] pub actor: String,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AssessmentIdInput { pub assessment_id: String }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListAssessmentsInput { pub subject: Option<String>, pub status: Option<AssessmentStatus> }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetAssessmentStatusInput { pub assessment_id: String, pub status: AssessmentStatus, #[serde(default = "dactor")] pub actor: String }

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartAttemptInput { pub assessment_id: String, pub learner_id: String, #[serde(default = "dactor")] pub actor: String }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResponseInput { pub item_id: String, pub value: serde_json::Value }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SubmitAttemptInput { pub attempt_id: String, pub responses: Vec<ResponseInput>, #[serde(default = "dactor")] pub actor: String }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ScoreRubricInput { pub attempt_id: String, pub item_id: String, pub criterion_scores: Vec<f64>, pub feedback: Option<String>, #[serde(default = "dactor")] pub actor: String }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AttemptIdInput { pub attempt_id: String }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LearnerIdInput { pub learner_id: String }
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AuditLogInput { #[serde(default = "dfifty")] pub limit: usize }
fn dfifty() -> usize { 50 }

// ─── server ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AssessmentServer { pub store: Arc<AssessmentStore> }

#[tool_router(server_handler)]
impl AssessmentServer {
    // standards / curriculum
    #[tool(description = "Add a learning standard (curriculum mapping target), e.g. a CCSS/NGSS code.")]
    fn add_standard(&self, Parameters(i): Parameters<AddStandardInput>) -> String {
        let s = self.store.add_standard(&i.code, &i.framework, &i.subject, &i.grade, &i.description, &i.actor);
        serde_json::to_string_pretty(&s).unwrap()
    }

    #[tool(description = "List learning standards, optionally by subject/framework.")]
    fn list_standards(&self, Parameters(i): Parameters<ListStandardsInput>) -> String {
        let v = self.store.list_standards(i.subject.as_deref(), i.framework.as_deref());
        serde_json::to_string_pretty(&serde_json::json!({"count": v.len(), "standards": v})).unwrap()
    }

    #[tool(description = "List items aligned to a standard — curriculum coverage for a standard.")]
    fn items_for_standard(&self, Parameters(i): Parameters<StandardIdInput>) -> String {
        let v = self.store.items_for_standard(&i.standard_id);
        serde_json::to_string_pretty(&serde_json::json!({"standard_id": i.standard_id, "count": v.len(), "items": v})).unwrap()
    }

    // rubrics
    #[tool(description = "Create a rubric with weighted criteria (for essay/constructed-response scoring).")]
    fn create_rubric(&self, Parameters(i): Parameters<CreateRubricInput>) -> String {
        let criteria = i.criteria.into_iter().map(|c| RubricCriterion { name: c.name, description: c.description, max_points: c.max_points }).collect();
        let r = self.store.create_rubric(&i.name, criteria, &i.actor);
        serde_json::to_string_pretty(&r).unwrap()
    }

    #[tool(description = "Get a rubric by id.")]
    fn get_rubric(&self, Parameters(i): Parameters<ItemIdInput>) -> String {
        match self.store.get_rubric(&i.item_id) {
            Some(r) => serde_json::to_string_pretty(&r).unwrap(), None => format!("Rubric not found: {}", i.item_id) }
    }

    // item bank
    #[tool(description = "Author an item (question). Types: multiple_choice / multiple_select (options + answer_key indices), true_false, short_answer, numeric (with tolerance), essay (needs rubric_id).")]
    fn create_item(&self, Parameters(i): Parameters<CreateItemInput>) -> String {
        match self.store.create_item(&i.stem, i.itype, &i.subject, i.difficulty, i.cognitive, i.points, i.options, i.answer_key, i.tolerance, i.standard_ids, i.rubric_id, &i.actor) {
            Ok(it) => serde_json::to_string_pretty(&it).unwrap(), Err(e) => format!("Error: {e}") }
    }

    #[tool(description = "Get an item by id (includes answer key — instructor-facing).")]
    fn get_item(&self, Parameters(i): Parameters<ItemIdInput>) -> String {
        match self.store.get_item(&i.item_id) {
            Some(it) => serde_json::to_string_pretty(&it).unwrap(), None => format!("Item not found: {}", i.item_id) }
    }

    #[tool(description = "Search the item bank by subject, difficulty, and/or standard.")]
    fn list_items(&self, Parameters(i): Parameters<ListItemsInput>) -> String {
        let v = self.store.list_items(i.subject.as_deref(), i.difficulty, i.standard_id.as_deref());
        serde_json::to_string_pretty(&serde_json::json!({"count": v.len(), "items": v})).unwrap()
    }

    // assessments
    #[tool(description = "Create an assessment from an explicit ordered list of item ids.")]
    fn create_assessment(&self, Parameters(i): Parameters<CreateAssessmentInput>) -> String {
        match self.store.create_assessment(&i.title, &i.subject, i.item_ids, i.pass_threshold, &i.actor) {
            Ok(a) => serde_json::to_string_pretty(&a).unwrap(), Err(e) => format!("Error: {e}") }
    }

    #[tool(description = "Generate an assessment from a blueprint (difficulty → count) drawing from the item bank, optionally constrained to standards. Powers the Quiz Generator.")]
    fn generate_assessment(&self, Parameters(i): Parameters<GenerateAssessmentInput>) -> String {
        let blueprint: Vec<(Difficulty, usize)> = i.blueprint.into_iter().map(|s| (s.difficulty, s.count)).collect();
        match self.store.generate_assessment(&i.title, &i.subject, &blueprint, &i.standard_ids, i.pass_threshold, &i.actor) {
            Ok(a) => serde_json::to_string_pretty(&a).unwrap(), Err(e) => format!("Error: {e}") }
    }

    #[tool(description = "Get an assessment by id.")]
    fn get_assessment(&self, Parameters(i): Parameters<AssessmentIdInput>) -> String {
        match self.store.get_assessment(&i.assessment_id) {
            Some(a) => serde_json::to_string_pretty(&a).unwrap(), None => format!("Assessment not found: {}", i.assessment_id) }
    }

    #[tool(description = "List assessments by subject/status.")]
    fn list_assessments(&self, Parameters(i): Parameters<ListAssessmentsInput>) -> String {
        let v = self.store.list_assessments(i.subject.as_deref(), i.status);
        serde_json::to_string_pretty(&serde_json::json!({"count": v.len(), "assessments": v})).unwrap()
    }

    #[tool(description = "Set assessment status: draft/published/archived. Publishing makes it live for learner attempts and is gated.")]
    fn set_assessment_status(&self, Parameters(i): Parameters<SetAssessmentStatusInput>) -> String {
        match self.store.set_assessment_status(&i.assessment_id, i.status, &i.actor) {
            Ok(a) => serde_json::to_string_pretty(&a).unwrap(), Err(e) => format!("Error: {e}") }
    }

    #[tool(description = "Blueprint summary of an assessment: item counts by difficulty and cognitive level, plus standards covered. For Instructor Copilot / Curriculum Mapping.")]
    fn assessment_blueprint(&self, Parameters(i): Parameters<AssessmentIdInput>) -> String {
        match self.store.assessment_blueprint(&i.assessment_id) {
            Some(v) => serde_json::to_string_pretty(&v).unwrap(), None => format!("Assessment not found: {}", i.assessment_id) }
    }

    // attempts & grading
    #[tool(description = "Start a learner attempt on a published assessment.")]
    fn start_attempt(&self, Parameters(i): Parameters<StartAttemptInput>) -> String {
        match self.store.start_attempt(&i.assessment_id, &i.learner_id, &i.actor) {
            Ok(a) => serde_json::to_string_pretty(&a).unwrap(), Err(e) => format!("Error: {e}") }
    }

    #[tool(description = "Submit attempt answers; auto-grades objective items. Essay/rubric items remain for score_rubric_item. Records a result — gated.")]
    fn submit_attempt(&self, Parameters(i): Parameters<SubmitAttemptInput>) -> String {
        let responses: Vec<(String, serde_json::Value)> = i.responses.into_iter().map(|r| (r.item_id, r.value)).collect();
        match self.store.submit_attempt(&i.attempt_id, responses, &i.actor) {
            Ok(a) => serde_json::to_string_pretty(&a).unwrap(), Err(e) => format!("Error: {e}") }
    }

    #[tool(description = "Score a rubric/essay item on an attempt with per-criterion scores (clamped to each criterion max). Powers the Rubric Grading Assistant. Finalizes the attempt when nothing remains ungraded.")]
    fn score_rubric_item(&self, Parameters(i): Parameters<ScoreRubricInput>) -> String {
        match self.store.score_rubric_item(&i.attempt_id, &i.item_id, i.criterion_scores, i.feedback, &i.actor) {
            Ok(a) => serde_json::to_string_pretty(&a).unwrap(), Err(e) => format!("Error: {e}") }
    }

    #[tool(description = "Get an attempt by id (responses, score, percent, pass/fail).")]
    fn get_attempt(&self, Parameters(i): Parameters<AttemptIdInput>) -> String {
        match self.store.get_attempt(&i.attempt_id) {
            Some(a) => serde_json::to_string_pretty(&a).unwrap(), None => format!("Attempt not found: {}", i.attempt_id) }
    }

    #[tool(description = "List attempts for an assessment.")]
    fn list_attempts(&self, Parameters(i): Parameters<AssessmentIdInput>) -> String {
        let v = self.store.attempts_for_assessment(&i.assessment_id);
        serde_json::to_string_pretty(&serde_json::json!({"count": v.len(), "attempts": v})).unwrap()
    }

    #[tool(description = "List a learner's attempts across assessments.")]
    fn learner_attempts(&self, Parameters(i): Parameters<LearnerIdInput>) -> String {
        let v = self.store.attempts_for_learner(&i.learner_id);
        serde_json::to_string_pretty(&serde_json::json!({"count": v.len(), "attempts": v})).unwrap()
    }

    // psychometrics & mastery
    #[tool(description = "Item analysis across graded attempts: per-item p-value (difficulty) and discrimination (top vs bottom third), with quality flags. For Instructor Copilot.")]
    fn item_analysis(&self, Parameters(i): Parameters<AssessmentIdInput>) -> String {
        match self.store.item_analysis(&i.assessment_id) {
            Some(v) => serde_json::to_string_pretty(&v).unwrap(), None => format!("Assessment not found: {}", i.assessment_id) }
    }

    #[tool(description = "Standard mastery for a learner: % of points earned per aligned standard across graded attempts, with level. Powers the Adaptive Tutor.")]
    fn standard_mastery(&self, Parameters(i): Parameters<LearnerIdInput>) -> String {
        serde_json::to_string_pretty(&self.store.standard_mastery(&i.learner_id)).unwrap()
    }

    #[tool(description = "Recent platform audit-trail entries (most recent first).")]
    fn audit_log(&self, Parameters(i): Parameters<AuditLogInput>) -> String {
        let v = self.store.audit_log(i.limit);
        serde_json::to_string_pretty(&serde_json::json!({"count": v.len(), "entries": v})).unwrap()
    }
}

#[async_trait::async_trait]
impl HealthCheck for AssessmentServer {
    async fn check_health(&self) -> HealthStatus {
        HealthStatus { healthy: true, message: Some("operational".into()), latency_ms: Some(1) }
    }
}
