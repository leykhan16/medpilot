use super::AiAnalyzer;
use crate::models::case::{AnalysisResult, LevelOfCare, MatchStrength, PossibleCondition};
use async_trait::async_trait;

pub struct MockAnalyzer;

impl MockAnalyzer {
    pub fn new() -> Self {
        MockAnalyzer
    }
}

struct Rule {
    keywords: &'static [&'static str],
    condition: &'static str,
    level: LevelOfCare,
    urgency: i16,
    confidence: f32,
}

const RULES: &[Rule] = &[
    Rule {
        keywords: &["chest pain", "can't breathe", "cannot breathe", "crushing pain"],
        condition: "Possible cardiac event",
        level: LevelOfCare::Emergency,
        urgency: 95,
        confidence: 0.7,
    },
    Rule {
        keywords: &["ear pain", "ear ache", "earache", "clogged ear"],
        condition: "Middle ear infection",
        level: LevelOfCare::ConsultDoctor,
        urgency: 30,
        confidence: 0.82,
    },
    Rule {
        keywords: &["fever", "cough", "sore throat"],
        condition: "Upper respiratory infection",
        level: LevelOfCare::ConsultDoctor,
        urgency: 45,
        confidence: 0.68,
    },
    Rule {
        keywords: &["headache", "migraine", "light sensitivity"],
        condition: "Tension headache / possible migraine",
        level: LevelOfCare::SelfCare,
        urgency: 20,
        confidence: 0.6,
    },
    Rule {
        keywords: &["stomach", "nausea", "vomiting", "diarrhea"],
        condition: "Gastrointestinal upset",
        level: LevelOfCare::SelfCare,
        urgency: 25,
        confidence: 0.55,
    },
];

#[async_trait]
impl AiAnalyzer for MockAnalyzer {
    async fn analyze(&self, messages: &[String]) -> anyhow::Result<AnalysisResult> {
        let transcript = messages.join(" ").to_lowercase();

        let mut matched: Vec<&Rule> = RULES
            .iter()
            .filter(|r| r.keywords.iter().any(|kw| transcript.contains(kw)))
            .collect();

        if matched.is_empty() {
            return Ok(AnalysisResult {
                level_of_care: LevelOfCare::ConsultDoctor,
                possible_conditions: vec![PossibleCondition {
                    name: "Unclear from symptoms described".to_string(),
                    match_strength: MatchStrength::Low,
                    supporting_symptoms: vec![],
                }],
                confidence: 0.3,
                urgency_score: 40,
                model_used: "mock-v1".to_string(),
            });
        }

        matched.sort_by_key(|r| std::cmp::Reverse(r.urgency));
        let primary = &matched[0];

        let possible_conditions = matched
            .iter()
            .map(|r| {
                let matched_keywords: Vec<String> = r
                    .keywords
                    .iter()
                    .filter(|kw| transcript.contains(*kw))
                    .map(|s| s.to_string())
                    .collect();
                PossibleCondition {
                    name: r.condition.to_string(),
                    match_strength: if r.urgency == primary.urgency {
                        MatchStrength::High
                    } else {
                        MatchStrength::Moderate
                    },
                    supporting_symptoms: matched_keywords,
                }
            })
            .collect();

        Ok(AnalysisResult {
            level_of_care: primary.level,
            possible_conditions,
            confidence: primary.confidence,
            urgency_score: primary.urgency,
            model_used: "mock-v1".to_string(),
        })
    }
}
