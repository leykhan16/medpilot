use crate::auth::{decode_token, Claims};
use crate::models::user::Role;
use crate::state::AppState;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
    RequestPartsExt,
};
use axum_extra::headers::{authorization::Bearer, Authorization};
use axum_extra::TypedHeader;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: Uuid,
    pub role: Role,
}

#[async_trait]
impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = crate::error::AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| crate::error::AppError::Unauthorized("missing or malformed bearer token".into()))?;

        let Claims { sub, role, .. } = decode_token(bearer.token(), &state.config.jwt_secret)
            .map_err(|_| crate::error::AppError::Unauthorized("invalid or expired token".into()))?;

        Ok(CurrentUser { id: sub, role })
    }
}

pub struct Clinician(pub CurrentUser);

#[async_trait]
impl FromRequestParts<AppState> for Clinician {
    type Rejection = crate::error::AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        match user.role {
            Role::Clinician | Role::Admin => Ok(Clinician(user)),
            _ => Err(crate::error::AppError::Forbidden("clinician access required".into())),
        }
    }
}

pub struct Patient(pub CurrentUser);

#[async_trait]
impl FromRequestParts<AppState> for Patient {
    type Rejection = crate::error::AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        match user.role {
            Role::Patient => Ok(Patient(user)),
            _ => Err(crate::error::AppError::Forbidden("patient access required".into())),
        }
    }
}
