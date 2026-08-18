use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum CaseStatus {
    Open,
    Analyzed,
    Booked,
    Closed,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Case {
    pub id: Uuid,
    pub patient_id: Uuid,
    pub status: CaseStatus,
    pub urgency_score: Option<i16>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MessageSender {
    Patient,
    Ai,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CaseMessage {
    pub id: Uuid,
    pub case_id: Uuid,
    pub sender: MessageSender,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PossibleCondition {
    pub name: String,
    pub match_strength: MatchStrength,
    pub supporting_symptoms: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MatchStrength {
    High,
    Moderate,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LevelOfCare {
    SelfCare,
    ConsultDoctor,
    UrgentCare,
    Emergency,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AiAnalysis {
    pub id: Uuid,
    pub case_id: Uuid,
    pub level_of_care: String,
    pub possible_conditions: Value,
    pub confidence: f32,
    pub model_used: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub level_of_care: LevelOfCare,
    pub possible_conditions: Vec<PossibleCondition>,
    pub confidence: f32,
    pub urgency_score: i16,
    pub model_used: String,
}
