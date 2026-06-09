//! Integration tests: item authoring/validation, blueprint generation, attempt
//! auto-grading across item types, rubric scoring, publish gating, and the
//! psychometric/mastery engines.

use mcp_assessment::store::AssessmentStore;
use mcp_assessment::types::*;

fn store() -> AssessmentStore {
    AssessmentStore::new()
}

#[test]
fn seed_has_item_bank_and_standards() {
    let s = store();
    assert!(s.list_items(None, None, None).len() >= 8);
    assert_eq!(s.list_standards(Some("Mathematics"), None).len(), 2);
}

#[test]
fn item_validation() {
    let s = store();
    // multiple_choice needs >=2 options and exactly one key
    assert!(s.create_item("q", ItemType::MultipleChoice, "M", Difficulty::Easy, Cognitive::Apply, 1.0, vec!["a".into()], vec!["0".into()], None, vec![], None, "t").is_err());
    // essay needs a rubric
    assert!(s.create_item("explain", ItemType::Essay, "M", Difficulty::Hard, Cognitive::Analyze, 5.0, vec![], vec![], None, vec![], None, "t").is_err());
    // out-of-range key index
    assert!(s.create_item("q", ItemType::MultipleChoice, "M", Difficulty::Easy, Cognitive::Apply, 1.0, vec!["a".into(), "b".into()], vec!["5".into()], None, vec![], None, "t").is_err());
    // valid
    assert!(s.create_item("q", ItemType::MultipleChoice, "M", Difficulty::Easy, Cognitive::Apply, 1.0, vec!["a".into(), "b".into()], vec!["1".into()], None, vec![], None, "t").is_ok());
}

#[test]
fn blueprint_generation_respects_counts() {
    let s = store();
    let bp = [(Difficulty::Easy, 2), (Difficulty::Medium, 2)];
    let a = s.generate_assessment("Quiz", "Mathematics", &bp, &[], 0.6, "t").unwrap();
    assert_eq!(a.item_ids.len(), 4);
    // asking for more than exist fails
    let bp2 = [(Difficulty::Hard, 99)];
    assert!(s.generate_assessment("Quiz2", "Mathematics", &bp2, &[], 0.6, "t").is_err());
}

#[test]
fn cannot_publish_empty_and_attempts_need_published() {
    let s = store();
    let a = s.create_assessment("Empty", "Mathematics", vec![], 0.6, "t").unwrap();
    assert!(s.set_assessment_status(&a.id, AssessmentStatus::Published, "t").is_err());
    // a real one, but not yet published
    let bp = [(Difficulty::Easy, 1)];
    let g = s.generate_assessment("Q", "Mathematics", &bp, &[], 0.6, "t").unwrap();
    assert!(s.start_attempt(&g.id, "L1", "t").is_err(), "draft assessment rejects attempts");
    s.set_assessment_status(&g.id, AssessmentStatus::Published, "t").unwrap();
    assert!(s.start_attempt(&g.id, "L1", "t").is_ok());
}

#[test]
fn auto_grade_objective_items() {
    let s = store();
    // Build an assessment of known objective items.
    let items = s.list_items(Some("Mathematics"), None, None);
    let numeric = items.iter().find(|i| matches!(i.itype, ItemType::Numeric) && i.stem.contains("2x + 3")).unwrap().clone();
    let mc = items.iter().find(|i| matches!(i.itype, ItemType::MultipleChoice)).unwrap().clone();
    let a = s.create_assessment("Obj", "Mathematics", vec![numeric.id.clone(), mc.id.clone()], 0.5, "t").unwrap();
    s.set_assessment_status(&a.id, AssessmentStatus::Published, "t").unwrap();
    let att = s.start_attempt(&a.id, "L1", "t").unwrap();
    // numeric answer 4 correct; mc key is index 2 (option "4" = x in x+5=9)
    let graded = s.submit_attempt(&att.id, vec![
        (numeric.id.clone(), serde_json::json!(4)),
        (mc.id.clone(), serde_json::json!("2")),
    ], "t").unwrap();
    assert_eq!(graded.status, AttemptStatus::Graded);
    assert_eq!(graded.score, Some(2.0));
    assert_eq!(graded.passed, Some(true));
}

#[test]
fn numeric_tolerance_and_wrong_answers() {
    let s = store();
    let items = s.list_items(Some("Mathematics"), None, None);
    let numeric = items.iter().find(|i| i.stem.contains("2x + 3")).unwrap().clone();
    let a = s.create_assessment("N", "Mathematics", vec![numeric.id.clone()], 0.6, "t").unwrap();
    s.set_assessment_status(&a.id, AssessmentStatus::Published, "t").unwrap();
    let att = s.start_attempt(&a.id, "L2", "t").unwrap();
    let graded = s.submit_attempt(&att.id, vec![(numeric.id.clone(), serde_json::json!(5))], "t").unwrap();
    assert_eq!(graded.score, Some(0.0));
    assert_eq!(graded.passed, Some(false));
}

#[test]
fn essay_needs_rubric_scoring_to_finalize() {
    let s = store();
    let items = s.list_items(Some("Mathematics"), None, None);
    let essay = items.iter().find(|i| matches!(i.itype, ItemType::Essay)).unwrap().clone();
    let a = s.create_assessment("E", "Mathematics", vec![essay.id.clone()], 0.6, "t").unwrap();
    s.set_assessment_status(&a.id, AssessmentStatus::Published, "t").unwrap();
    let att = s.start_attempt(&a.id, "L3", "t").unwrap();
    // submitting an essay leaves the attempt un-finalized (Submitted, not Graded)
    let sub = s.submit_attempt(&att.id, vec![(essay.id.clone(), serde_json::json!("Because the parabola crosses the axis twice."))], "t").unwrap();
    assert_eq!(sub.status, AttemptStatus::Submitted);
    assert!(sub.score.is_none());
    // rubric: criteria are [3,2]; award 3 + 2 = 5 of 5
    let graded = s.score_rubric_item(&att.id, &essay.id, vec![3.0, 2.0], Some("Great".into()), "grader").unwrap();
    assert_eq!(graded.status, AttemptStatus::Graded);
    assert_eq!(graded.score, Some(5.0));
    assert_eq!(graded.passed, Some(true));
}

#[test]
fn rubric_scores_clamped_to_criteria_max() {
    let s = store();
    let items = s.list_items(Some("Mathematics"), None, None);
    let essay = items.iter().find(|i| matches!(i.itype, ItemType::Essay)).unwrap().clone();
    let a = s.create_assessment("E", "Mathematics", vec![essay.id.clone()], 0.6, "t").unwrap();
    s.set_assessment_status(&a.id, AssessmentStatus::Published, "t").unwrap();
    let att = s.start_attempt(&a.id, "L4", "t").unwrap();
    s.submit_attempt(&att.id, vec![(essay.id.clone(), serde_json::json!("answer"))], "t").unwrap();
    // over-max scores clamp to [3,2] = 5
    let graded = s.score_rubric_item(&att.id, &essay.id, vec![99.0, 99.0], None, "g").unwrap();
    assert_eq!(graded.score, Some(5.0));
}

#[test]
fn item_analysis_and_mastery() {
    let s = store();
    let items = s.list_items(Some("Mathematics"), None, None);
    let n1 = items.iter().find(|i| i.stem.contains("2x + 3")).unwrap().clone();
    let mc = items.iter().find(|i| matches!(i.itype, ItemType::MultipleChoice)).unwrap().clone();
    let a = s.create_assessment("Analytics", "Mathematics", vec![n1.id.clone(), mc.id.clone()], 0.5, "t").unwrap();
    s.set_assessment_status(&a.id, AssessmentStatus::Published, "t").unwrap();
    // two learners: one all-correct, one all-wrong
    let a1 = s.start_attempt(&a.id, "good", "t").unwrap();
    s.submit_attempt(&a1.id, vec![(n1.id.clone(), serde_json::json!(4)), (mc.id.clone(), serde_json::json!("2"))], "t").unwrap();
    let a2 = s.start_attempt(&a.id, "weak", "t").unwrap();
    s.submit_attempt(&a2.id, vec![(n1.id.clone(), serde_json::json!(0)), (mc.id.clone(), serde_json::json!("3"))], "t").unwrap();

    let ia = s.item_analysis(&a.id).unwrap();
    assert_eq!(ia["graded_attempts"], 2);
    let rows = ia["items"].as_array().unwrap();
    // each item: 1 of 2 correct -> p_value 0.5
    assert!(rows.iter().all(|r| r["p_value"].as_f64().unwrap() == 0.5));

    let mastery = s.standard_mastery("good");
    let stds = mastery["standards"].as_array().unwrap();
    assert!(!stds.is_empty());
    // good learner earned full points on aligned standards -> proficient
    assert!(stds.iter().any(|x| x["level"] == "proficient"));
}
