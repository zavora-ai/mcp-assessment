//! In-memory assessment store with seeded data and engines.
//!
//! Thread-safe via per-collection `Mutex`. IDs come from a monotonic sequence
//! (`PREFIX-{n}` from 1000). Every state change appends to an audit trail.
//! Engines: blueprint-driven generation, auto-grading (objective + rubric),
//! item analysis (difficulty p-value + discrimination), and standard mastery.

use crate::types::*;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct AssessmentStore {
    items: Mutex<HashMap<String, Item>>,
    standards: Mutex<HashMap<String, Standard>>,
    rubrics: Mutex<HashMap<String, Rubric>>,
    assessments: Mutex<HashMap<String, Assessment>>,
    attempts: Mutex<HashMap<String, Attempt>>,
    audit_log: Mutex<Vec<AuditEntry>>,
    seq: Mutex<u64>,
}

impl Default for AssessmentStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AssessmentStore {
    pub fn new() -> Self {
        let s = AssessmentStore {
            items: Mutex::new(HashMap::new()),
            standards: Mutex::new(HashMap::new()),
            rubrics: Mutex::new(HashMap::new()),
            assessments: Mutex::new(HashMap::new()),
            attempts: Mutex::new(HashMap::new()),
            audit_log: Mutex::new(Vec::new()),
            seq: Mutex::new(1000),
        };
        s.seed();
        s
    }

    fn next(&self, prefix: &str) -> String {
        let mut n = self.seq.lock().unwrap();
        *n += 1;
        format!("{prefix}-{n}")
    }

    fn audit(&self, actor: &str, action: &str, detail: impl Into<String>) {
        self.audit_log.lock().unwrap().push(AuditEntry {
            at: Utc::now(),
            actor: actor.to_string(),
            action: action.to_string(),
            detail: detail.into(),
        });
    }

    // ─── standards (curriculum mapping) ────────────────────────────────────

    pub fn add_standard(&self, code: &str, framework: &str, subject: &str, grade: &str, description: &str, actor: &str) -> Standard {
        let s = Standard { id: self.next("STD"), code: code.to_string(), framework: framework.to_string(), subject: subject.to_string(), grade: grade.to_string(), description: description.to_string() };
        self.standards.lock().unwrap().insert(s.id.clone(), s.clone());
        self.audit(actor, "add_standard", s.code.clone());
        s
    }

    pub fn list_standards(&self, subject: Option<&str>, framework: Option<&str>) -> Vec<Standard> {
        let mut v: Vec<Standard> = self.standards.lock().unwrap().values()
            .filter(|s| subject.is_none_or(|x| s.subject.eq_ignore_ascii_case(x)))
            .filter(|s| framework.is_none_or(|x| s.framework.eq_ignore_ascii_case(x)))
            .cloned().collect();
        v.sort_by(|a, b| a.code.cmp(&b.code));
        v
    }

    // ─── rubrics ───────────────────────────────────────────────────────────

    pub fn create_rubric(&self, name: &str, criteria: Vec<RubricCriterion>, actor: &str) -> Rubric {
        let r = Rubric { id: self.next("RUB"), name: name.to_string(), criteria, created_at: Utc::now() };
        self.rubrics.lock().unwrap().insert(r.id.clone(), r.clone());
        self.audit(actor, "create_rubric", r.id.clone());
        r
    }

    pub fn get_rubric(&self, id: &str) -> Option<Rubric> {
        self.rubrics.lock().unwrap().get(id).cloned()
    }

    // ─── item bank ─────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn create_item(&self, stem: &str, itype: ItemType, subject: &str, difficulty: Difficulty, cognitive: Cognitive, points: f64, options: Vec<String>, answer_key: Vec<String>, tolerance: Option<f64>, standard_ids: Vec<String>, rubric_id: Option<String>, actor: &str) -> Result<Item, String> {
        // Validate by type.
        match itype {
            ItemType::MultipleChoice | ItemType::MultipleSelect if options.len() < 2 =>
                return Err("choice items need at least 2 options".into()),
            ItemType::MultipleChoice if answer_key.len() != 1 =>
                return Err("multiple_choice needs exactly one correct option".into()),
            ItemType::MultipleSelect | ItemType::MultipleChoice if answer_key.is_empty() =>
                return Err("choice items need an answer key".into()),
            ItemType::TrueFalse if answer_key.len() != 1 =>
                return Err("true_false needs exactly one answer (true/false)".into()),
            ItemType::Numeric if answer_key.len() != 1 =>
                return Err("numeric needs exactly one answer value".into()),
            ItemType::Essay if rubric_id.is_none() =>
                return Err("essay items require a rubric_id".into()),
            _ => {}
        }
        // Validate choice answer-key indices.
        if matches!(itype, ItemType::MultipleChoice | ItemType::MultipleSelect) {
            for k in &answer_key {
                let idx: usize = k.parse().map_err(|_| format!("answer key '{k}' must be an option index"))?;
                if idx >= options.len() { return Err(format!("answer key index {idx} out of range")); }
            }
        }
        if let Some(rid) = &rubric_id {
            if self.get_rubric(rid).is_none() { return Err(format!("Rubric not found: {rid}")); }
        }
        let item = Item {
            id: self.next("ITM"),
            stem: stem.to_string(),
            itype,
            subject: subject.to_string(),
            difficulty,
            cognitive,
            points,
            options,
            answer_key,
            tolerance,
            standard_ids,
            rubric_id,
            created_by: actor.to_string(),
            created_at: Utc::now(),
        };
        self.items.lock().unwrap().insert(item.id.clone(), item.clone());
        self.audit(actor, "create_item", item.id.clone());
        Ok(item)
    }

    pub fn get_item(&self, id: &str) -> Option<Item> {
        self.items.lock().unwrap().get(id).cloned()
    }

    pub fn list_items(&self, subject: Option<&str>, difficulty: Option<Difficulty>, standard_id: Option<&str>) -> Vec<Item> {
        let mut v: Vec<Item> = self.items.lock().unwrap().values()
            .filter(|i| subject.is_none_or(|x| i.subject.eq_ignore_ascii_case(x)))
            .filter(|i| difficulty.is_none_or(|d| i.difficulty == d))
            .filter(|i| standard_id.is_none_or(|s| i.standard_ids.iter().any(|x| x == s)))
            .cloned().collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    /// Items aligned to a standard — the curriculum-coverage query.
    pub fn items_for_standard(&self, standard_id: &str) -> Vec<Item> {
        self.list_items(None, None, Some(standard_id))
    }

    // ─── assessments ─────────────────────────────────────────────────────

    pub fn create_assessment(&self, title: &str, subject: &str, item_ids: Vec<String>, pass_threshold: f64, actor: &str) -> Result<Assessment, String> {
        let items = self.items.lock().unwrap();
        let mut total = 0.0;
        for id in &item_ids {
            let it = items.get(id).ok_or_else(|| format!("Item not found: {id}"))?;
            total += it.points;
        }
        drop(items);
        let a = Assessment {
            id: self.next("ASM"),
            title: title.to_string(),
            subject: subject.to_string(),
            status: AssessmentStatus::Draft,
            item_ids,
            total_points: total,
            pass_threshold: pass_threshold.clamp(0.0, 1.0),
            created_by: actor.to_string(),
            created_at: Utc::now(),
            published_at: None,
        };
        self.assessments.lock().unwrap().insert(a.id.clone(), a.clone());
        self.audit(actor, "create_assessment", a.id.clone());
        Ok(a)
    }

    /// Blueprint-driven generation (Quiz Generator): pick items from the bank
    /// matching subject and a difficulty mix, optionally constrained to standards.
    /// `blueprint` maps difficulty → count.
    pub fn generate_assessment(&self, title: &str, subject: &str, blueprint: &[(Difficulty, usize)], standard_ids: &[String], pass_threshold: f64, actor: &str) -> Result<Assessment, String> {
        let mut chosen: Vec<String> = Vec::new();
        let pool = self.list_items(Some(subject), None, None);
        for (diff, count) in blueprint {
            let mut candidates: Vec<&Item> = pool.iter()
                .filter(|i| i.difficulty == *diff)
                .filter(|i| standard_ids.is_empty() || i.standard_ids.iter().any(|s| standard_ids.contains(s)))
                .filter(|i| !chosen.contains(&i.id))
                .collect();
            if candidates.len() < *count {
                return Err(format!("blueprint needs {count} {diff:?} items but only {} available in '{subject}'", candidates.len()));
            }
            // Deterministic: take the first N by id (stable selection).
            candidates.sort_by(|a, b| a.id.cmp(&b.id));
            for it in candidates.into_iter().take(*count) {
                chosen.push(it.id.clone());
            }
        }
        if chosen.is_empty() { return Err("blueprint produced no items".into()); }
        let a = self.create_assessment(title, subject, chosen, pass_threshold, actor)?;
        self.audit(actor, "generate_assessment", format!("{} ({} items)", a.id, a.item_ids.len()));
        Ok(a)
    }

    pub fn get_assessment(&self, id: &str) -> Option<Assessment> {
        self.assessments.lock().unwrap().get(id).cloned()
    }

    pub fn list_assessments(&self, subject: Option<&str>, status: Option<AssessmentStatus>) -> Vec<Assessment> {
        let mut v: Vec<Assessment> = self.assessments.lock().unwrap().values()
            .filter(|a| subject.is_none_or(|x| a.subject.eq_ignore_ascii_case(x)))
            .filter(|a| status.is_none_or(|s| a.status == s))
            .cloned().collect();
        v.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        v
    }

    /// Set assessment status. Publishing requires at least one item.
    pub fn set_assessment_status(&self, id: &str, status: AssessmentStatus, actor: &str) -> Result<Assessment, String> {
        let mut asms = self.assessments.lock().unwrap();
        let a = asms.get_mut(id).ok_or_else(|| format!("Assessment not found: {id}"))?;
        if status == AssessmentStatus::Published && a.item_ids.is_empty() {
            return Err("cannot publish an assessment with no items".into());
        }
        if status == AssessmentStatus::Published { a.published_at = Some(Utc::now()); }
        a.status = status;
        let out = a.clone();
        drop(asms);
        self.audit(actor, "set_assessment_status", format!("{id} -> {status:?}"));
        Ok(out)
    }

    /// Blueprint summary of an assessment: counts by difficulty/cognitive level
    /// and the standards it covers — for the Instructor Copilot / Curriculum agent.
    pub fn assessment_blueprint(&self, id: &str) -> Option<serde_json::Value> {
        let a = self.get_assessment(id)?;
        let items = self.items.lock().unwrap();
        let mut by_diff: HashMap<String, u64> = HashMap::new();
        let mut by_cog: HashMap<String, u64> = HashMap::new();
        let mut standards: HashMap<String, u64> = HashMap::new();
        for id in &a.item_ids {
            if let Some(it) = items.get(id) {
                *by_diff.entry(format!("{:?}", it.difficulty).to_lowercase()).or_insert(0) += 1;
                *by_cog.entry(format!("{:?}", it.cognitive).to_lowercase()).or_insert(0) += 1;
                for s in &it.standard_ids { *standards.entry(s.clone()).or_insert(0) += 1; }
            }
        }
        Some(serde_json::json!({
            "assessment_id": a.id,
            "title": a.title,
            "items": a.item_ids.len(),
            "total_points": a.total_points,
            "by_difficulty": by_diff,
            "by_cognitive": by_cog,
            "standards_covered": standards,
        }))
    }

    // ─── attempts & grading ──────────────────────────────────────────────

    pub fn start_attempt(&self, assessment_id: &str, learner_id: &str, actor: &str) -> Result<Attempt, String> {
        let a = self.get_assessment(assessment_id).ok_or_else(|| format!("Assessment not found: {assessment_id}"))?;
        if a.status != AssessmentStatus::Published {
            return Err(format!("Assessment {assessment_id} is {:?}; only published assessments accept attempts", a.status));
        }
        let attempt = Attempt {
            id: self.next("ATT"),
            assessment_id: assessment_id.to_string(),
            learner_id: learner_id.to_string(),
            status: AttemptStatus::InProgress,
            responses: Vec::new(),
            score: None,
            max_score: a.total_points,
            percent: None,
            passed: None,
            started_at: Utc::now(),
            submitted_at: None,
            graded_at: None,
        };
        self.attempts.lock().unwrap().insert(attempt.id.clone(), attempt.clone());
        self.audit(actor, "start_attempt", format!("{} {}", attempt.id, learner_id));
        Ok(attempt)
    }

    /// Submit answers and auto-grade objective items. Essay/rubric items are left
    /// ungraded (awarded_points=None) for `score_rubric_item`.
    pub fn submit_attempt(&self, attempt_id: &str, responses: Vec<(String, serde_json::Value)>, actor: &str) -> Result<Attempt, String> {
        let mut attempts = self.attempts.lock().unwrap();
        let att = attempts.get_mut(attempt_id).ok_or_else(|| format!("Attempt not found: {attempt_id}"))?;
        if att.status == AttemptStatus::Graded {
            return Err(format!("Attempt {attempt_id} already graded"));
        }
        let items = self.items.lock().unwrap();
        let mut graded = Vec::new();
        let mut needs_manual = false;
        for (item_id, value) in responses {
            let it = items.get(&item_id).ok_or_else(|| format!("Item not found: {item_id}"))?;
            let (pts, correct) = grade_objective(it, &value);
            if pts.is_none() { needs_manual = true; }
            graded.push(ItemResponse {
                item_id,
                value,
                awarded_points: pts,
                correct,
                rubric_scores: Vec::new(),
                feedback: None,
            });
        }
        att.responses = graded;
        att.status = AttemptStatus::Submitted;
        att.submitted_at = Some(Utc::now());
        // If everything auto-graded, finalize now.
        if !needs_manual {
            let threshold = self.assessments.lock().unwrap().get(&att.assessment_id).map(|a| a.pass_threshold).unwrap_or(0.6);
            finalize(att, threshold);
        }
        let out = att.clone();
        drop(items);
        drop(attempts);
        self.audit(actor, "submit_attempt", format!("{attempt_id} -> {:?}", out.status));
        Ok(out)
    }

    /// Score a rubric/essay item on an attempt (Rubric Grading Assistant).
    pub fn score_rubric_item(&self, attempt_id: &str, item_id: &str, criterion_scores: Vec<f64>, feedback: Option<String>, actor: &str) -> Result<Attempt, String> {
        let item = self.get_item(item_id).ok_or_else(|| format!("Item not found: {item_id}"))?;
        let rubric_id = item.rubric_id.clone().ok_or_else(|| format!("Item {item_id} has no rubric"))?;
        let rubric = self.get_rubric(&rubric_id).ok_or("rubric missing")?;
        if criterion_scores.len() != rubric.criteria.len() {
            return Err(format!("expected {} criterion scores, got {}", rubric.criteria.len(), criterion_scores.len()));
        }
        // Clamp each criterion to its max and sum.
        let mut awarded = 0.0;
        for (score, crit) in criterion_scores.iter().zip(&rubric.criteria) {
            awarded += score.clamp(0.0, crit.max_points);
        }
        let mut attempts = self.attempts.lock().unwrap();
        let att = attempts.get_mut(attempt_id).ok_or_else(|| format!("Attempt not found: {attempt_id}"))?;
        let resp = att.responses.iter_mut().find(|r| r.item_id == item_id).ok_or_else(|| format!("No response for item {item_id} on this attempt"))?;
        resp.rubric_scores = criterion_scores;
        resp.awarded_points = Some(awarded);
        resp.correct = Some(awarded >= item.points * 0.6);
        resp.feedback = feedback;
        // Finalize if no responses remain ungraded.
        if att.responses.iter().all(|r| r.awarded_points.is_some()) {
            let threshold = self.assessments.lock().unwrap().get(&att.assessment_id).map(|a| a.pass_threshold).unwrap_or(0.6);
            finalize(att, threshold);
        }
        let out = att.clone();
        drop(attempts);
        self.audit(actor, "score_rubric_item", format!("{attempt_id}/{item_id} +{awarded}"));
        Ok(out)
    }

    pub fn get_attempt(&self, id: &str) -> Option<Attempt> {
        self.attempts.lock().unwrap().get(id).cloned()
    }

    pub fn attempts_for_assessment(&self, assessment_id: &str) -> Vec<Attempt> {
        let mut v: Vec<Attempt> = self.attempts.lock().unwrap().values().filter(|a| a.assessment_id == assessment_id).cloned().collect();
        v.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        v
    }

    pub fn attempts_for_learner(&self, learner_id: &str) -> Vec<Attempt> {
        let mut v: Vec<Attempt> = self.attempts.lock().unwrap().values().filter(|a| a.learner_id == learner_id).cloned().collect();
        v.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        v
    }

    // ─── psychometrics & mastery ─────────────────────────────────────────

    /// Item analysis across graded attempts of an assessment: per-item p-value
    /// (difficulty = proportion correct) and discrimination (point-biserial-ish:
    /// top-third vs bottom-third correct-rate difference).
    pub fn item_analysis(&self, assessment_id: &str) -> Option<serde_json::Value> {
        let a = self.get_assessment(assessment_id)?;
        let attempts: Vec<Attempt> = self.attempts.lock().unwrap().values()
            .filter(|t| t.assessment_id == assessment_id && t.status == AttemptStatus::Graded)
            .cloned().collect();
        let n = attempts.len();
        if n == 0 {
            return Some(serde_json::json!({"assessment_id": assessment_id, "graded_attempts": 0, "items": []}));
        }
        // Rank attempts by total percent for discrimination groups.
        let mut ranked = attempts.clone();
        ranked.sort_by(|x, y| y.percent.unwrap_or(0.0).partial_cmp(&x.percent.unwrap_or(0.0)).unwrap());
        let group = (n as f64 / 3.0).ceil() as usize;
        let top = &ranked[..group.min(n)];
        let bottom = &ranked[n.saturating_sub(group)..];

        let correct_rate = |set: &[Attempt], item_id: &str| -> Option<f64> {
            let rs: Vec<bool> = set.iter().filter_map(|t| t.responses.iter().find(|r| r.item_id == item_id)).filter_map(|r| r.correct).collect();
            if rs.is_empty() { None } else { Some(rs.iter().filter(|&&c| c).count() as f64 / rs.len() as f64) }
        };

        let mut rows = Vec::new();
        for item_id in &a.item_ids {
            let p = correct_rate(&attempts, item_id);
            let disc = match (correct_rate(top, item_id), correct_rate(bottom, item_id)) {
                (Some(t), Some(b)) => Some((t - b).clamp(-1.0, 1.0)),
                _ => None,
            };
            let flag = match (p, disc) {
                (Some(p), _) if p < 0.2 => "too_hard",
                (Some(p), _) if p > 0.9 => "too_easy",
                (_, Some(d)) if d < 0.1 => "low_discrimination",
                _ => "ok",
            };
            rows.push(serde_json::json!({
                "item_id": item_id,
                "p_value": p.map(|x| (x*1000.0).round()/1000.0),
                "discrimination": disc.map(|x| (x*1000.0).round()/1000.0),
                "flag": flag,
            }));
        }
        Some(serde_json::json!({"assessment_id": assessment_id, "graded_attempts": n, "items": rows}))
    }

    /// Standard mastery for a learner: proportion of points earned on items
    /// aligned to each standard across their graded attempts. Powers the Adaptive
    /// Tutor and Curriculum Mapping agents.
    pub fn standard_mastery(&self, learner_id: &str) -> serde_json::Value {
        let attempts: Vec<Attempt> = self.attempts.lock().unwrap().values()
            .filter(|a| a.learner_id == learner_id && a.status == AttemptStatus::Graded)
            .cloned().collect();
        let items = self.items.lock().unwrap();
        // standard_id -> (earned, possible)
        let mut agg: HashMap<String, (f64, f64)> = HashMap::new();
        for att in &attempts {
            for r in &att.responses {
                if let Some(it) = items.get(&r.item_id) {
                    let earned = r.awarded_points.unwrap_or(0.0);
                    for sid in &it.standard_ids {
                        let e = agg.entry(sid.clone()).or_insert((0.0, 0.0));
                        e.0 += earned;
                        e.1 += it.points;
                    }
                }
            }
        }
        let standards = self.standards.lock().unwrap();
        let mut rows: Vec<serde_json::Value> = agg.iter().map(|(sid, (earned, possible))| {
            let pct = if *possible > 0.0 { (earned / possible * 1000.0).round() / 10.0 } else { 0.0 };
            let level = if pct >= 80.0 { "proficient" } else if pct >= 60.0 { "developing" } else { "beginning" };
            serde_json::json!({
                "standard_id": sid,
                "code": standards.get(sid).map(|s| s.code.clone()),
                "earned": (earned*100.0).round()/100.0,
                "possible": possible,
                "mastery_pct": pct,
                "level": level,
            })
        }).collect();
        rows.sort_by(|a, b| a["mastery_pct"].as_f64().partial_cmp(&b["mastery_pct"].as_f64()).unwrap());
        serde_json::json!({"learner_id": learner_id, "graded_attempts": attempts.len(), "standards": rows})
    }

    pub fn audit_log(&self, limit: usize) -> Vec<AuditEntry> {
        let log = self.audit_log.lock().unwrap();
        log.iter().rev().take(limit).cloned().collect()
    }

    // ─── seed ────────────────────────────────────────────────────────────

    fn seed(&self) {
        // Standards (curriculum).
        let s_lin = self.add_standard("CCSS.MATH.HSA.REI.B.3", "CCSS", "Mathematics", "9", "Solve linear equations and inequalities in one variable", "system");
        let s_quad = self.add_standard("CCSS.MATH.HSA.REI.B.4", "CCSS", "Mathematics", "9", "Solve quadratic equations in one variable", "system");

        // A rubric for essay/constructed-response scoring.
        let rubric = self.create_rubric("Short Explanation Rubric", vec![
            RubricCriterion { name: "Correctness".into(), description: "Mathematically correct".into(), max_points: 3.0 },
            RubricCriterion { name: "Reasoning".into(), description: "Clear justification".into(), max_points: 2.0 },
        ], "system");

        // Item bank — a mix of types/difficulties aligned to the two standards.
        self.create_item("Solve: 2x + 3 = 11", ItemType::Numeric, "Mathematics", Difficulty::Easy, Cognitive::Apply, 1.0, vec![], vec!["4".into()], Some(0.001), vec![s_lin.id.clone()], None, "system").unwrap();
        self.create_item("Which is a solution to x + 5 = 9?", ItemType::MultipleChoice, "Mathematics", Difficulty::Easy, Cognitive::Remember, 1.0, vec!["2".into(), "3".into(), "4".into(), "5".into()], vec!["2".into()], None, vec![s_lin.id.clone()], None, "system").unwrap();
        self.create_item("Is x = -3 a solution of x^2 = 9?", ItemType::TrueFalse, "Mathematics", Difficulty::Medium, Cognitive::Understand, 1.0, vec![], vec!["true".into()], None, vec![s_quad.id.clone()], None, "system").unwrap();
        self.create_item("Select all roots of x^2 - 5x + 6 = 0", ItemType::MultipleSelect, "Mathematics", Difficulty::Medium, Cognitive::Apply, 2.0, vec!["1".into(), "2".into(), "3".into(), "6".into()], vec!["1".into(), "2".into()], None, vec![s_quad.id.clone()], None, "system").unwrap();
        self.create_item("Solve x^2 = 49 for the positive root", ItemType::Numeric, "Mathematics", Difficulty::Medium, Cognitive::Apply, 1.0, vec![], vec!["7".into()], Some(0.001), vec![s_quad.id.clone()], None, "system").unwrap();
        self.create_item("Explain why a quadratic can have two solutions", ItemType::Essay, "Mathematics", Difficulty::Hard, Cognitive::Analyze, 5.0, vec![], vec![], None, vec![s_quad.id.clone()], Some(rubric.id.clone()), "system").unwrap();
        self.create_item("What is the slope of y = 3x + 2?", ItemType::ShortAnswer, "Mathematics", Difficulty::Easy, Cognitive::Remember, 1.0, vec![], vec!["3".into()], None, vec![s_lin.id.clone()], None, "system").unwrap();
        self.create_item("Solve: 5x - 10 = 0", ItemType::Numeric, "Mathematics", Difficulty::Hard, Cognitive::Apply, 1.0, vec![], vec!["2".into()], Some(0.001), vec![s_lin.id.clone()], None, "system").unwrap();
    }
}

// ─── grading helpers ─────────────────────────────────────────────────────

/// Grade an objective item. Returns (awarded_points, correct). For items that
/// need manual/rubric scoring (essay), returns (None, None).
fn grade_objective(item: &Item, value: &serde_json::Value) -> (Option<f64>, Option<bool>) {
    match item.itype {
        ItemType::Essay => (None, None),
        ItemType::MultipleChoice | ItemType::TrueFalse => {
            let given = match item.itype {
                ItemType::TrueFalse => value.as_bool().map(|b| b.to_string()).or_else(|| value.as_str().map(|s| s.to_lowercase())),
                _ => value.as_str().map(|s| s.to_string()).or_else(|| value.as_i64().map(|n| n.to_string())),
            };
            match given {
                Some(g) => {
                    let correct = item.answer_key.iter().any(|k| k.eq_ignore_ascii_case(&g));
                    (Some(if correct { item.points } else { 0.0 }), Some(correct))
                }
                None => (Some(0.0), Some(false)),
            }
        }
        ItemType::MultipleSelect => {
            let given: std::collections::HashSet<String> = value.as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string()))).collect())
                .unwrap_or_default();
            let key: std::collections::HashSet<String> = item.answer_key.iter().cloned().collect();
            let correct = given == key;
            (Some(if correct { item.points } else { 0.0 }), Some(correct))
        }
        ItemType::ShortAnswer => {
            let g = value.as_str().unwrap_or("").trim().to_lowercase();
            let correct = item.answer_key.iter().any(|k| k.trim().to_lowercase() == g);
            (Some(if correct { item.points } else { 0.0 }), Some(correct))
        }
        ItemType::Numeric => {
            let g = value.as_f64().or_else(|| value.as_str().and_then(|s| s.parse().ok()));
            let key = item.answer_key.first().and_then(|k| k.parse::<f64>().ok());
            match (g, key) {
                (Some(g), Some(k)) => {
                    let tol = item.tolerance.unwrap_or(0.0);
                    let correct = (g - k).abs() <= tol;
                    (Some(if correct { item.points } else { 0.0 }), Some(correct))
                }
                _ => (Some(0.0), Some(false)),
            }
        }
    }
}

/// Compute score/percent/passed from graded responses and mark the attempt graded.
fn finalize(att: &mut Attempt, pass_threshold: f64) {
    let score: f64 = att.responses.iter().filter_map(|r| r.awarded_points).sum();
    att.score = Some((score * 100.0).round() / 100.0);
    let frac = if att.max_score > 0.0 { score / att.max_score } else { 0.0 };
    att.percent = Some((frac * 1000.0).round() / 10.0);
    att.passed = Some(frac >= pass_threshold);
    att.status = AttemptStatus::Graded;
    att.graded_at = Some(Utc::now());
}
