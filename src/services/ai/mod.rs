use crate::models::case::AnalysisResult;
use async_trait::async_trait;

pub mod mock;

#[async_trait]
pub trait AiAnalyzer: Send + Sync {
    async fn analyze(&self, messages: &[String]) -> anyhow::Result<AnalysisResult>;
}
