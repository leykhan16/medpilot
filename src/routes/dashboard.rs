use crate::error::AppResult;
use crate::extractors::Clinician;
use crate::state::AppState;
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct DashboardSummary {
    pub active_cases: i64,
    pub ai_analyses_run: i64,
    pub unacknowledged_alerts: i64,
    pub upcoming_appointments: i64,
}

pub async fn summary(
    State(state): State<AppState>,
    Clinician(_user): Clinician,
) -> AppResult<Json<DashboardSummary>> {
    let active_cases = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM cases WHERE status != 'closed'"
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    let ai_analyses_run = sqlx::query_scalar!("SELECT COUNT(*) FROM ai_analyses")
        .fetch_one(&state.db)
        .await?
        .unwrap_or(0);

    let unacknowledged_alerts = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM alerts WHERE acknowledged = false"
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    let upcoming_appointments = sqlx::query_scalar!(
        r#"SELECT COUNT(*) FROM appointments WHERE status = 'scheduled' AND scheduled_at > now()"#
    )
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0);

    Ok(Json(DashboardSummary {
        active_cases,
        ai_analyses_run,
        unacknowledged_alerts,
        upcoming_appointments,
    }))
}
