use crate::error::{AppError, AppResult};
use crate::extractors::{Clinician, Patient};
use crate::models::appointment::{Alert, Appointment, AppointmentStatus, BookAppointmentRequest};
use crate::routes::cases::ensure_owns_case;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

pub async fn book_appointment(
    State(state): State<AppState>,
    Patient(user): Patient,
    Path(case_id): Path<Uuid>,
    Json(req): Json<BookAppointmentRequest>,
) -> AppResult<Json<Appointment>> {
    ensure_owns_case(&state, user.id, case_id).await?;

    if req.scheduled_at < chrono::Utc::now() {
        return Err(AppError::BadRequest("scheduled_at must be in the future".into()));
    }

    let mut tx = state.db.begin().await?;

    let appointment = sqlx::query_as!(
        Appointment,
        r#"
        INSERT INTO appointments (case_id, clinician_id, scheduled_at)
        VALUES ($1, $2, $3)
        RETURNING id, case_id, clinician_id, scheduled_at, status AS "status: AppointmentStatus", created_at
        "#,
        case_id,
        req.clinician_id,
        req.scheduled_at,
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"UPDATE cases SET status = 'booked', updated_at = now() WHERE id = $1"#,
        case_id,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(appointment))
}

pub async fn list_appointments(
    State(state): State<AppState>,
    Clinician(_user): Clinician,
) -> AppResult<Json<Vec<Appointment>>> {
    let appointments = sqlx::query_as!(
        Appointment,
        r#"SELECT id, case_id, clinician_id, scheduled_at, status AS "status: AppointmentStatus", created_at
           FROM appointments ORDER BY scheduled_at"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(appointments))
}

pub async fn list_alerts(
    State(state): State<AppState>,
    Clinician(_user): Clinician,
) -> AppResult<Json<Vec<Alert>>> {
    let alerts = sqlx::query_as!(
        Alert,
        r#"SELECT id, case_id, reason, acknowledged, created_at
           FROM alerts WHERE acknowledged = false ORDER BY created_at DESC"#
    )
    .fetch_all(&state.db)
    .await?;

    Ok(Json(alerts))
}

pub async fn acknowledge_alert(
    State(state): State<AppState>,
    Clinician(_user): Clinician,
    Path(alert_id): Path<Uuid>,
) -> AppResult<Json<Alert>> {
    let alert = sqlx::query_as!(
        Alert,
        r#"
        UPDATE alerts SET acknowledged = true
        WHERE id = $1
        RETURNING id, case_id, reason, acknowledged, created_at
        "#,
        alert_id,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::NotFound("alert not found".into()))?;

    Ok(Json(alert))
}
