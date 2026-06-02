use crate::{QwenError, language_models::LLMError};
use serde_json::Value;
use std::str::from_utf8;

/// Parse error from JSON response and return appropriate QwenError
pub(crate) fn parse_error_response(code: &str, message: &str) -> LLMError {
    match code {
        // 400 errors
        "InvalidParameter" | "invalid_parameter_error" => {
            LLMError::QwenError(QwenError::InvalidParameterError(message.to_string()))
        }
        "APIConnectionError" => {
            LLMError::QwenError(QwenError::APIConnectionError(message.to_string()))
        }

        // 401 errors
        "InvalidApiKey" => LLMError::QwenError(QwenError::InvalidApiKeyError(message.to_string())),

        // 429 errors
        "ModelServingError" => {
            LLMError::QwenError(QwenError::ModelServingError(message.to_string()))
        }
        "PrepaidBillOverdue" => {
            LLMError::QwenError(QwenError::PrepaidBillOverdueError(message.to_string()))
        }
        "PostpaidBillOverdue" => {
            LLMError::QwenError(QwenError::PostpaidBillOverdueError(message.to_string()))
        }
        "CommodityNotPurchased" => {
            LLMError::QwenError(QwenError::CommodityNotPurchasedError(message.to_string()))
        }

        // 500 errors
        "InternalError" | "internal_error" => {
            LLMError::QwenError(QwenError::InternalError(message.to_string()))
        }
        "InternalError.Algo" => {
            LLMError::QwenError(QwenError::InternalAlgorithmError(message.to_string()))
        }
        "InternalError.Timeout" => {
            LLMError::QwenError(QwenError::TimeoutError(message.to_string()))
        }
        "RewriteFailed" => LLMError::QwenError(QwenError::RewriteFailedError(message.to_string())),
        "RetrivalFailed" => {
            LLMError::QwenError(QwenError::RetrievalFailedError(message.to_string()))
        }
        "AppProcessFailed" => {
            LLMError::QwenError(QwenError::AppProcessFailedError(message.to_string()))
        }
        "ModelServiceFailed" => {
            LLMError::QwenError(QwenError::ModelServiceFailedError(message.to_string()))
        }
        "InvokePluginFailed" => {
            LLMError::QwenError(QwenError::InvokePluginFailedError(message.to_string()))
        }
        "SystemError" | "system_error" => {
            LLMError::QwenError(QwenError::SystemError(message.to_string()))
        }

        // 503 errors
        "ModelUnavailable" => {
            LLMError::QwenError(QwenError::ModelUnavailableError(message.to_string()))
        }

        // Other errors
        "mismatched_model" => {
            LLMError::QwenError(QwenError::MismatchedModelError(message.to_string()))
        }
        "duplicate_custom_id" => {
            LLMError::QwenError(QwenError::DuplicateCustomIdError(message.to_string()))
        }
        "model_not_found" => {
            LLMError::QwenError(QwenError::ModelNotFoundError(message.to_string()))
        }

        // Default error
        _ => LLMError::QwenError(QwenError::SystemError(format!(
            "Unknown error code: {}, message: {}",
            code, message
        ))),
    }
}

/// Parse Server-Sent Events (SSE) chunks
pub(crate) fn parse_sse_chunk(bytes: &[u8]) -> Result<Vec<Value>, LLMError> {
    let text = from_utf8(bytes).map_err(|e| LLMError::OtherError(e.to_string()))?;
    let mut values = Vec::new();

    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                continue;
            }

            match serde_json::from_str::<Value>(data) {
                Ok(value) => values.push(value),
                Err(e) => {
                    return Err(LLMError::OtherError(format!(
                        "Failed to parse SSE data: {}, data: {}",
                        e, data
                    )));
                }
            }
        }
    }

    Ok(values)
}
