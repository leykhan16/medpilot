use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AppointmentStatus {
    Scheduled,
    Completed,
    Cancelled,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Appointment {
    pub id: Uuid,
    pub case_id: Uuid,
    pub clinician_id: Option<Uuid>,
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
    pub status: AppointmentStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BookAppointmentRequest {
    pub clinician_id: Option<Uuid>,
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Alert {
    pub id: Uuid,
    pub case_id: Uuid,
    pub reason: String,
    pub acknowledged: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
