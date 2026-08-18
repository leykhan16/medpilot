use crate::config::Config;
use crate::services::ai::AiAnalyzer;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub ai: Arc<dyn AiAnalyzer>,
}
