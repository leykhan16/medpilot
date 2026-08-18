use crate::error::{AppError, AppResult};
use crate::extractors::{Clinician, Patient};
use crate::models::case::{
    AiAnalysis, Case, CaseMessage, CaseStatus, MessageSender, SendMessageRequest,
};
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

/// POST /cases  (patient only)
/// Opens a new case for the calling patient. We look up patient_profiles.id
/// from the JWT's user id rather than trusting a patient_id in the request
/// body — otherwise any patient could open a case under someone else's name.
pub async fn create_case(
    State(state): State<AppState>,
    Patient(user): Patient,
) -> AppResult<Json<Case>> {
    let patient_id = sqlx::query_scalar!(
        "SELECT id FROM patient_profiles WHERE user_id = $1",
        user.id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("patient profile not found".into()))?;

    let case = sqlx::query_as!(
        Case,
        r#"
        INSERT INTO cases (patient_id)
        VALUES ($1)
        RETURNING id, patient_id, status AS "status: CaseStatus", urgency_score, created_at, updated_at
        "#,
        patient_id,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(case))
}

/// POST /cases/:id/messages  (patient only)
/// Appends a message to the case transcript. Ownership check: the case's
/// patient_id must belong to the calling user, or any patient could post
/// into anyone else's case by guessing a UUID.
pub async fn send_message(
    State(state): State<AppState>,
    Patient(user): Patient,
    Path(case_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> AppResult<Json<CaseMessage>> {
    ensure_owns_case(&state, user.id, case_id).await?;

    let message = sqlx::query_as!(
        CaseMessage,
        r#"
        INSERT INTO case_messages (case_id, sender, content)
        VALUES ($1, 'patient', $2)
        RETURNING id, case_id, sender AS "sender: MessageSender", content, created_at
        "#,
        case_id,
        req.content,
    )
    .fetch_one(&state.db)
    .await?;

    Ok(Json(message))
}

const URGENCY_ALERT_THRESHOLD: i16 = 70;

#[derive(Serialize)]
pub struct AnalyzeResponse {
    pub analysis: AiAnalysis,
    pub alert_raised: bool,
}

/// POST /cases/:id/analyze  (patient only)
/// Pulls the full transcript, runs it through whichever AiAnalyzer is wired
/// in AppState (MockAnalyzer today), persists the result, and flips the
/// case to 'analyzed'. If urgency clears the threshold, also writes an
/// alert row.
pub async fn analyze_case(
    State(state): State<AppState>,
    Patient(user): Patient,
    Path(case_id): Path<Uuid>,
) -> AppResult<Json<AnalyzeResponse>> {
    ensure_owns_case(&state, user.id, case_id).await?;

    let messages: Vec<String> = sqlx::query_scalar!(
        "SELECT content FROM case_messages WHERE case_id = $1 ORDER BY created_at",
        case_id
    )
    .fetch_all(&state.db)
    .await?;

    if messages.is_empty() {
        return Err(AppError::BadRequest(
            "case has no messages yet — send at least one before analyzing".into(),
        ));
    }

    let result = state
        .ai
        .analyze(&messages)
        .await
        .map_err(AppError::Internal)?;

    let conditions_json = serde_json::to_value(&result.possible_conditions)
        .map_err(|e| AppError::Internal(e.into()))?;
    let level_str = serde_json::to_value(&result.level_of_care)
        .map_err(|e| AppError::Internal(e.into()))?
        .as_str()
        .unwrap_or("consult_doctor")
        .to_string();

    let mut tx = state.db.begin().await?;

    let analysis = sqlx::query_as!(
        AiAnalysis,
        r#"
        INSERT INTO ai_analyses (case_id, level_of_care, possible_conditions, confidence, model_used)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (case_id) DO UPDATE
        SET level_of_care = EXCLUDED.level_of_care,
            possible_conditions = EXCLUDED.possible_conditions,
            confidence = EXCLUDED.confidence,
            model_used = EXCLUDED.model_used,
            created_at = now()
        RETURNING id, case_id, level_of_care, possible_conditions, confidence, model_used, created_at
        "#,
        case_id,
        level_str,
        conditions_json,
        result.confidence,
        result.model_used,
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"UPDATE cases SET status = 'analyzed', urgency_score = $1, updated_at = now() WHERE id = $2"#,
        result.urgency_score,
        case_id,
    )
    .execute(&mut *tx)
    .await?;

    let alert_raised = result.urgency_score >= URGENCY_ALERT_THRESHOLD;
    if alert_raised {
        sqlx::query!(
            "INSERT INTO alerts (case_id, reason) VALUES ($1, $2)",
            case_id,
            format!(
                "Urgency score {} — {}",
                result.urgency_score,
                result
                    .possible_conditions
                    .first()
                    .map(|c| c.name.as_str())
                    .unwrap_or("unspecified")
            ),
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(Json(AnalyzeResponse { analysis, alert_raised }))
}

/// GET /cases/:id  (patient who owns it, or any clinician)
pub async fn get_case(
    State(state): State<AppState>,
    Path(case_id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let case = sqlx::query_as!(
        Case,
        r#"SELECT id, patient_id, status AS "status: CaseStatus", urgency_score, created_at, updated_at
           FROM cases WHERE id = $1"#,
        case_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("case not found".into()))?;

    let messages = sqlx::query_as!(
        CaseMessage,
        r#"SELECT id, case_id, sender AS "sender: MessageSender", content, created_at
           FROM case_messages WHERE case_id = $1 ORDER BY created_at"#,
        case_id
    )
    .fetch_all(&state.db)
    .await?;

    let analysis = sqlx::query_as!(
        AiAnalysis,
        r#"SELECT id, case_id, level_of_care, possible_conditions, confidence, model_used, created_at
           FROM ai_analyses WHERE case_id = $1"#,
        case_id
    )
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(json!({
        "case": case,
        "messages": messages,
        "analysis": analysis,
    })))
}

/// Shared ownership check used by every patient-scoped case route.
/// NOTE: currently patient-only — clinicians only get read access via
/// get_case in this round.
pub(crate) async fn ensure_owns_case(state: &AppState, user_id: Uuid, case_id: Uuid) -> AppResult<()> {
    let owner_user_id = sqlx::query_scalar!(
        r#"
        SELECT pp.user_id
        FROM cases c
        JOIN patient_profiles pp ON pp.id = c.patient_id
        WHERE c.id = $1
        "#,
        case_id
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("case not found".into()))?;

    if owner_user_id != user_id {
        return Err(AppError::Forbidden("this case does not belong to you".into()));
    }
    Ok(())
}

/// GET /cases  (clinician only) — dashboard's "active cases" list.
pub async fn list_cases(
    State(state): State<AppState>,
    Clinician(_user): Clinician,
) -> AppResult<Json<Vec<Case>>> {
    let cases = sqlx::query_as!(
        Case,
        r#"SELECT id, patient_id, status AS "status: CaseStatus", urgency_score, created_at, updated_at
           FROM cases ORDER BY created_at DESC"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(cases))
}
