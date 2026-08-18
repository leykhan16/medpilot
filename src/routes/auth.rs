use crate::auth::{create_token, hash_password, verify_password};
use crate::error::{AppError, AppResult};
use crate::models::user::{AuthResponse, LoginRequest, RegisterRequest, Role, User};
use crate::state::AppState;
use axum::{extract::State, Json};

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> AppResult<Json<AuthResponse>> {
    if req.password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 characters".into()));
    }

    let existing = sqlx::query_scalar!("SELECT id FROM users WHERE email = $1", req.email)
        .fetch_optional(&state.db)
        .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("an account with that email already exists".into()));
    }

    let password_hash = hash_password(&req.password)?;

    let mut tx = state.db.begin().await?;

    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (email, password_hash, role, name)
        VALUES ($1, $2, $3, $4)
        RETURNING id, email, password_hash, role AS "role: Role", name, created_at
        "#,
        req.email,
        password_hash,
        req.role as Role,
        req.name,
    )
    .fetch_one(&mut *tx)
    .await?;

    if req.role == Role::Patient {
        sqlx::query!(
            "INSERT INTO patient_profiles (user_id) VALUES ($1)",
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let token = create_token(user.id, user.role, &state.config.jwt_secret)?;
    Ok(Json(AuthResponse { token, user }))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, email, password_hash, role AS "role: Role", name, created_at
           FROM users WHERE email = $1"#,
        req.email,
    )
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::Unauthorized("invalid email or password".into()))?;

    let valid = verify_password(&req.password, &user.password_hash)?;
    if !valid {
        return Err(AppError::Unauthorized("invalid email or password".into()));
    }

    let token = create_token(user.id, user.role, &state.config.jwt_secret)?;
    Ok(Json(AuthResponse { token, user }))
}

pub async fn me(crate::extractors::CurrentUser { id, role }: crate::extractors::CurrentUser) -> AppResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "id": id, "role": role })))
}
