use crate::summary::SummaryTextStreamCallback;
use futures_util::StreamExt;
use reqwest::{header, Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::info;

// Generic structure for OpenAI-compatible API chat messages
#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

// Generic structure for OpenAI-compatible API chat requests
#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

// Generic structure for OpenAI-compatible API chat responses
#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
pub struct Choice {
    pub message: MessageContent,
}

#[derive(Deserialize, Debug)]
pub struct MessageContent {
    pub content: String,
}

// Claude-specific request structure
#[derive(Debug, Serialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

// Claude-specific response structure
#[derive(Deserialize, Debug)]
pub struct ClaudeChatResponse {
    pub content: Vec<ClaudeChatContent>,
}

#[derive(Deserialize, Debug)]
pub struct ClaudeChatContent {
    pub text: String,
}

/// LLM Provider enumeration for multi-provider support
#[derive(Debug, Clone, PartialEq)]
pub enum LLMProvider {
    OpenAI,
    Claude,
    Groq,
    Ollama,
    OpenRouter,
    BuiltInAI,
    CustomOpenAI,
}

impl LLMProvider {
    /// Parse provider from string (case-insensitive)
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Self::OpenAI),
            "claude" => Ok(Self::Claude),
            "groq" => Ok(Self::Groq),
            "ollama" => Ok(Self::Ollama),
            "openrouter" => Ok(Self::OpenRouter),
            "builtin-ai" | "local-llama" | "localllama" => Ok(Self::BuiltInAI),
            "custom-openai" => Ok(Self::CustomOpenAI),
            _ => Err(format!("Unsupported LLM provider: {}", s)),
        }
    }
}

/// Generates a summary using the specified LLM provider
///
/// # Arguments
/// * `client` - Reqwest HTTP client (reused for performance)
/// * `provider` - The LLM provider to use
/// * `model_name` - The specific model to use (e.g., "gpt-4", "claude-3-opus")
/// * `api_key` - API key for the provider (not needed for Ollama)
/// * `system_prompt` - System instructions for the LLM
/// * `user_prompt` - User query/content to process
/// * `ollama_endpoint` - Optional custom Ollama endpoint (defaults to localhost:11434)
/// * `custom_openai_endpoint` - Optional custom OpenAI-compatible endpoint
/// * `max_tokens` - Optional max tokens (for CustomOpenAI provider)
/// * `temperature` - Optional temperature (for CustomOpenAI provider)
/// * `top_p` - Optional top_p (for CustomOpenAI provider)
/// * `app_data_dir` - Optional app data directory (for BuiltInAI provider)
/// * `cancellation_token` - Optional token to cancel the request
///
/// # Returns
/// The generated summary text or an error message
pub async fn generate_summary(
    client: &Client,
    provider: &LLMProvider,
    model_name: &str,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    ollama_endpoint: Option<&str>,
    custom_openai_endpoint: Option<&str>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    request_timeout: Duration,
    app_data_dir: Option<&PathBuf>,
    cancellation_token: Option<&CancellationToken>,
) -> Result<String, String> {
    generate_summary_with_callback(
        client,
        provider,
        model_name,
        api_key,
        system_prompt,
        user_prompt,
        ollama_endpoint,
        custom_openai_endpoint,
        max_tokens,
        temperature,
        top_p,
        request_timeout,
        app_data_dir,
        cancellation_token,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn generate_summary_with_callback(
    client: &Client,
    provider: &LLMProvider,
    model_name: &str,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
    ollama_endpoint: Option<&str>,
    custom_openai_endpoint: Option<&str>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    request_timeout: Duration,
    app_data_dir: Option<&PathBuf>,
    cancellation_token: Option<&CancellationToken>,
    stream_callback: Option<&SummaryTextStreamCallback>,
) -> Result<String, String> {
    // Check if cancelled before starting
    if let Some(token) = cancellation_token {
        if token.is_cancelled() {
            return Err("Summary generation was cancelled".to_string());
        }
    }

    // Handle BuiltInAI provider separately (uses local sidecar, no HTTP API)
    if provider == &LLMProvider::BuiltInAI {
        let app_data_dir = app_data_dir
            .ok_or_else(|| "app_data_dir is required for BuiltInAI provider".to_string())?;

        return crate::summary::summary_engine::generate_with_builtin_streaming(
            app_data_dir,
            model_name,
            system_prompt,
            user_prompt,
            cancellation_token,
            stream_callback,
            request_timeout,
        )
        .await
        .map_err(|e| e.to_string());
    }

    let (api_url, mut headers) = match provider {
        LLMProvider::OpenAI => (
            "https://api.openai.com/v1/chat/completions".to_string(),
            header::HeaderMap::new(),
        ),
        LLMProvider::Groq => (
            "https://api.groq.com/openai/v1/chat/completions".to_string(),
            header::HeaderMap::new(),
        ),
        LLMProvider::OpenRouter => (
            "https://openrouter.ai/api/v1/chat/completions".to_string(),
            header::HeaderMap::new(),
        ),
        LLMProvider::Ollama => {
            let host = ollama_endpoint
                .map(|s| s.to_string())
                .unwrap_or_else(|| "http://localhost:11434".to_string());
            (
                format!("{}/v1/chat/completions", host),
                header::HeaderMap::new(),
            )
        }
        LLMProvider::CustomOpenAI => {
            let endpoint = custom_openai_endpoint
                .ok_or_else(|| "Custom OpenAI endpoint not configured".to_string())?;
            (
                format!("{}/chat/completions", endpoint.trim_end_matches('/')),
                header::HeaderMap::new(),
            )
        }
        LLMProvider::Claude => {
            let mut header_map = header::HeaderMap::new();
            header_map.insert(
                "x-api-key",
                api_key
                    .parse()
                    .map_err(|_| "Invalid API key format".to_string())?,
            );
            header_map.insert(
                "anthropic-version",
                "2023-06-01"
                    .parse()
                    .map_err(|_| "Invalid anthropic version".to_string())?,
            );
            (
                "https://api.anthropic.com/v1/messages".to_string(),
                header_map,
            )
        }
        LLMProvider::BuiltInAI => {
            // This case is handled earlier with early returns
            unreachable!("BuiltInAI is handled before this match statement")
        }
    };

    // Add authorization header for non-Claude providers
    if provider != &LLMProvider::Claude {
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {}", api_key)
                .parse()
                .map_err(|_| "Invalid authorization header".to_string())?,
        );
    }
    headers.insert(
        header::CONTENT_TYPE,
        "application/json"
            .parse()
            .map_err(|_| "Invalid content type".to_string())?,
    );

    // Build request body based on provider
    let request_body = if provider != &LLMProvider::Claude {
        // For CustomOpenAI, apply optional parameters if provided
        let (max_tokens_val, temperature_val, top_p_val) = if provider == &LLMProvider::CustomOpenAI
        {
            (max_tokens, temperature, top_p)
        } else {
            (None, None, None)
        };

        serde_json::json!(ChatRequest {
            model: model_name.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                }
            ],
            max_tokens: max_tokens_val,
            temperature: temperature_val,
            top_p: top_p_val,
            stream: stream_callback.map(|_| true),
        })
    } else {
        serde_json::json!(ClaudeRequest {
            system: system_prompt.to_string(),
            model: model_name.to_string(),
            max_tokens: 2048,
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            }],
            stream: stream_callback.map(|_| true),
        })
    };

    info!(
        "🐞 LLM Request to {}: model={}",
        provider_name(provider),
        model_name
    );

    // Send request with timeout and cancellation support
    let request_future = client
        .post(api_url)
        .headers(headers)
        .json(&request_body)
        .timeout(request_timeout)
        .send();

    // Use tokio::select to race between cancellation and request completion
    let response = if let Some(token) = cancellation_token {
        tokio::select! {
            result = request_future => {
                result.map_err(|e| {
                    if e.is_timeout() {
                        format!(
                            "LLM request timed out after {} seconds",
                            request_timeout.as_secs()
                        )
                    } else {
                        format!("Failed to send request to LLM: {}", e)
                    }
                })?
            }
            _ = token.cancelled() => {
                return Err("Summary generation was cancelled".to_string());
            }
        }
    } else {
        request_future.await.map_err(|e| {
            if e.is_timeout() {
                format!(
                    "LLM request timed out after {} seconds",
                    request_timeout.as_secs()
                )
            } else {
                format!("Failed to send request to LLM: {}", e)
            }
        })?
    };

    if !response.status().is_success() {
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("LLM API request failed: {}", error_body));
    }

    if let Some(callback) = stream_callback {
        return parse_streaming_response(response, provider, callback, cancellation_token).await;
    }

    // Parse response based on provider
    if provider == &LLMProvider::Claude {
        let chat_response = response
            .json::<ClaudeChatResponse>()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        info!("🐞 LLM Response received from Claude");

        let content = chat_response
            .content
            .get(0)
            .ok_or("No content in LLM response")?
            .text
            .trim();
        Ok(content.to_string())
    } else {
        let chat_response = response
            .json::<ChatResponse>()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        info!("🐞 LLM Response received from {}", provider_name(provider));

        let content = chat_response
            .choices
            .get(0)
            .ok_or("No content in LLM response")?
            .message
            .content
            .trim();
        Ok(content.to_string())
    }
}

async fn parse_streaming_response(
    response: Response,
    provider: &LLMProvider,
    callback: &SummaryTextStreamCallback,
    cancellation_token: Option<&CancellationToken>,
) -> Result<String, String> {
    let mut byte_stream = response.bytes_stream();
    let mut pending = Vec::new();
    let mut raw_body = Vec::new();
    let mut accumulated = String::new();

    loop {
        let next_chunk = if let Some(token) = cancellation_token {
            tokio::select! {
                chunk = byte_stream.next() => chunk,
                _ = token.cancelled() => {
                    return Err("Summary generation was cancelled".to_string());
                }
            }
        } else {
            byte_stream.next().await
        };

        let Some(chunk_result) = next_chunk else {
            break;
        };
        let chunk = chunk_result.map_err(|e| format!("Failed to read LLM stream: {e}"))?;
        raw_body.extend_from_slice(&chunk);
        pending.extend_from_slice(&chunk);

        while let Some(newline_index) = pending.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = pending.drain(..=newline_index).collect();
            append_stream_line(&line, provider, &mut accumulated, callback)?;
        }
    }

    if !pending.is_empty() {
        append_stream_line(&pending, provider, &mut accumulated, callback)?;
    }

    if accumulated.trim().is_empty() {
        let raw_text = String::from_utf8(raw_body)
            .map_err(|e| format!("LLM response was not valid UTF-8: {e}"))?;
        let complete = parse_complete_response(&raw_text, provider)?;
        if !complete.is_empty() {
            callback(complete.clone());
        }
        return Ok(complete);
    }

    Ok(accumulated.trim().to_string())
}

fn append_stream_line(
    line: &[u8],
    provider: &LLMProvider,
    accumulated: &mut String,
    callback: &SummaryTextStreamCallback,
) -> Result<(), String> {
    let line = std::str::from_utf8(line)
        .map_err(|e| format!("LLM stream contained invalid UTF-8: {e}"))?
        .trim();

    if line.is_empty()
        || line.starts_with(':')
        || line.starts_with("event:")
        || line.starts_with("id:")
    {
        return Ok(());
    }

    let payload = line.strip_prefix("data:").map(str::trim).unwrap_or(line);
    if payload.is_empty() || payload == "[DONE]" {
        return Ok(());
    }

    let Ok(value) = serde_json::from_str::<Value>(payload) else {
        // Some OpenAI-compatible endpoints ignore `stream: true` and return a
        // pretty-printed JSON body. The complete body is parsed after EOF.
        return Ok(());
    };

    if let Some(error) = value.get("error") {
        return Err(format!("LLM stream returned an error: {error}"));
    }

    if let Some(delta) = extract_stream_delta(&value, provider) {
        if !delta.is_empty() {
            accumulated.push_str(delta);
            callback(accumulated.clone());
        }
    }

    Ok(())
}

fn extract_stream_delta<'a>(value: &'a Value, provider: &LLMProvider) -> Option<&'a str> {
    if provider == &LLMProvider::Claude {
        return value
            .get("delta")
            .and_then(|delta| delta.get("text"))
            .and_then(Value::as_str)
            .or_else(|| {
                value
                    .get("content")
                    .and_then(Value::as_array)
                    .and_then(|content| content.first())
                    .and_then(|item| item.get("text"))
                    .and_then(Value::as_str)
            });
    }

    value
        .pointer("/choices/0/delta/content")
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
        })
        .or_else(|| value.pointer("/choices/0/text").and_then(Value::as_str))
        .or_else(|| value.pointer("/message/content").and_then(Value::as_str))
        .or_else(|| value.get("response").and_then(Value::as_str))
}

fn parse_complete_response(body: &str, provider: &LLMProvider) -> Result<String, String> {
    if provider == &LLMProvider::Claude {
        let response = serde_json::from_str::<ClaudeChatResponse>(body)
            .map_err(|e| format!("Failed to parse Claude response: {e}"))?;
        return response
            .content
            .first()
            .map(|content| content.text.trim().to_string())
            .ok_or_else(|| "No content in LLM response".to_string());
    }

    if let Ok(response) = serde_json::from_str::<ChatResponse>(body) {
        return response
            .choices
            .first()
            .map(|choice| choice.message.content.trim().to_string())
            .ok_or_else(|| "No content in LLM response".to_string());
    }

    let value = serde_json::from_str::<Value>(body)
        .map_err(|e| format!("Failed to parse LLM response: {e}"))?;
    extract_stream_delta(&value, provider)
        .map(|content| content.trim().to_string())
        .ok_or_else(|| "No content in LLM response".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn extracts_openai_stream_delta() {
        let value: Value =
            serde_json::from_str(r#"{"choices":[{"delta":{"content":"Hello"}}]}"#).unwrap();
        assert_eq!(
            extract_stream_delta(&value, &LLMProvider::OpenAI),
            Some("Hello")
        );
    }

    #[test]
    fn extracts_claude_stream_delta() {
        let value: Value = serde_json::from_str(
            r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"你好"}}"#,
        )
        .unwrap();
        assert_eq!(
            extract_stream_delta(&value, &LLMProvider::Claude),
            Some("你好")
        );
    }

    #[test]
    fn extracts_ollama_ndjson_delta() {
        let value: Value =
            serde_json::from_str(r#"{"message":{"content":"world"},"done":false}"#).unwrap();
        assert_eq!(
            extract_stream_delta(&value, &LLMProvider::Ollama),
            Some("world")
        );
    }

    #[test]
    fn streaming_lines_emit_complete_snapshots() {
        let snapshots = Arc::new(Mutex::new(Vec::new()));
        let captured = snapshots.clone();
        let callback: SummaryTextStreamCallback = Arc::new(move |snapshot| {
            captured.lock().unwrap().push(snapshot);
        });
        let mut accumulated = String::new();

        append_stream_line(
            br#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#,
            &LLMProvider::OpenAI,
            &mut accumulated,
            &callback,
        )
        .unwrap();
        append_stream_line(
            br#"data: {"choices":[{"delta":{"content":" world"}}]}"#,
            &LLMProvider::OpenAI,
            &mut accumulated,
            &callback,
        )
        .unwrap();

        assert_eq!(accumulated, "Hello world");
        assert_eq!(
            *snapshots.lock().unwrap(),
            vec!["Hello".to_string(), "Hello world".to_string()]
        );
    }
}

/// Helper function to get provider name for logging
fn provider_name(provider: &LLMProvider) -> &str {
    match provider {
        LLMProvider::OpenAI => "OpenAI",
        LLMProvider::Claude => "Claude",
        LLMProvider::Groq => "Groq",
        LLMProvider::Ollama => "Ollama",
        LLMProvider::BuiltInAI => "Built-in AI",
        LLMProvider::OpenRouter => "OpenRouter",
        LLMProvider::CustomOpenAI => "Custom OpenAI",
    }
}
