use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::{ModelRequest, ModelResponse, StreamEvent};

/// A unified interface for model provider SDKs.
///
/// Implementations handle the specifics of each provider SDK while exposing a
/// common completion and completion-stream API.
#[async_trait]
pub trait ModelProviderSDK: Send + Sync {
    /// Send a request and get a complete response.
    async fn completion(&self, request: ModelRequest) -> anyhow::Result<ModelResponse>;

    /// Send a request and get a stream of incremental events.
    ///
    /// Dropping the returned stream should cancel the in-flight request and
    /// close the underlying transport if the provider supports streaming.
    async fn completion_stream(
        &self,
        request: ModelRequest,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<StreamEvent>> + Send>>>;

    /// Human-readable provider name (e.g. "anthropic", "openai").
    fn name(&self) -> &str;

    /// Backward-compatible alias for `completion`.
    async fn complete(&self, request: ModelRequest) -> anyhow::Result<ModelResponse> {
        self.completion(request).await
    }

    /// Backward-compatible alias for `completion_stream`.
    async fn stream(
        &self,
        request: ModelRequest,
    ) -> anyhow::Result<Pin<Box<dyn Stream<Item = anyhow::Result<StreamEvent>> + Send>>> {
        self.completion_stream(request).await
    }
}

/// Backward-compatible alias for the provider SDK trait.
pub use ModelProviderSDK as ModelProvider;
