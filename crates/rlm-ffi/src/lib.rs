//! WASM FFI bindings for Cherry Studio.

#![forbid(unsafe_code)]

use rlm_core::{RlmConfig, RlmExecutor, RlmRequest};
use rlm_repl_rhai::RhaiReplBackend;
use wasm_bindgen::prelude::*;

// MockLlmProvider is not available in production builds of rlm-core/test-utils by default
// For this phase, we will implement a basic JS-delegating provider or use a todo! panic
// until Phase 12 (Polyglot) provides the bridging infrastructure.
// For now, to ensure compilation, we will use a struct that implements the generic requirements if possible,
// or stub it out as the plan suggested "MockLlmProvider" which implies it might be copy-pasted or
// we need to implement a simple one here.

// Since the user wants to "Proceed with Phase 6", I will enable it to compile.
// I'll create a basic JsLlmProvider that errors out for now, to satisfy the types.

use async_trait::async_trait;
use futures::Stream;
use rlm_core::ports::{LlmProvider, ModelInfo, ProviderMetadata};
use rlm_core::types::{ChatCompletionRequest, ChatCompletionResponse};
use std::pin::Pin;

#[derive(Debug, Clone)]
struct JsLlmProvider;

#[async_trait]
impl LlmProvider for JsLlmProvider {
    async fn complete(
        &self,
        _request: &ChatCompletionRequest,
    ) -> rlm_core::error::RlmResult<ChatCompletionResponse> {
        // TODO: Call back into JS via a provided function or fetch()
        Err(rlm_core::error::RlmError::NotImplemented(
            "WASM LLM Provider not yet implemented".to_string(),
        ))
    }

    async fn complete_stream(
        &self,
        _request: &ChatCompletionRequest,
    ) -> rlm_core::error::RlmResult<
        Pin<Box<dyn Stream<Item = rlm_core::error::RlmResult<ChatCompletionResponse>> + Send>>,
    > {
        Err(rlm_core::error::RlmError::NotImplemented(
            "WASM LLM streaming not yet implemented".to_string(),
        ))
    }

    async fn list_models(&self) -> rlm_core::error::RlmResult<Vec<ModelInfo>> {
        Ok(vec![ModelInfo::new(
            "wasm-placeholder",
            "WASM Placeholder Model",
            8192,
            4096,
        )])
    }

    fn validate_request(&self, _request: &ChatCompletionRequest) -> rlm_core::error::RlmResult<()> {
        Ok(())
    }

    fn get_metadata(&self) -> ProviderMetadata {
        ProviderMetadata::new("WASM JS Bridge", "0.1.0", "wasm://localhost", false)
    }

    async fn health_check(&self) -> rlm_core::error::RlmResult<bool> {
        Ok(true)
    }
}

/// Execute RLM request from JavaScript.
#[wasm_bindgen]
pub async fn rlm_execute(
    request_json: JsValue,
    _event_callback: js_sys::Function,
) -> Result<JsValue, JsValue> {
    // Parse request
    let request: RlmRequest = serde_wasm_bindgen::from_value(request_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Create executor
    let config = RlmConfig::default();
    let repl = RhaiReplBackend::new();
    let llm = JsLlmProvider;

    let executor = RlmExecutor::new(config, repl, llm);

    // TODO: Wire event_callback to event sink
    // The plan had this TOD, so we keep it.
    // Ideally we'd wrap event_callback in a struct implementing EventSink and pass it.

    let response = executor
        .execute(request)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
}
