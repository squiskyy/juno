use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::models::OllamaModel;

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaTagResponse {
    pub models: Vec<OllamaTagModel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaTagModel {
    pub name: String,
    pub model: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
    pub details: OllamaModelDetails,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OllamaModelDetails {
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub families: Option<Vec<String>>,
    #[serde(default)]
    pub parameter_size: String,
    #[serde(default)]
    pub quantization_level: String,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ChatStreamChunk {
    pub message: Option<OllamaMessage>,
    pub done: bool,
    #[serde(default)]
    pub done_reason: String,
}

pub struct OllamaClient {
    base_url: String,
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    pub fn set_base_url(&mut self, url: &str) {
        self.base_url = url.trim_end_matches('/').to_string();
    }

    pub async fn list_models(&self) -> anyhow::Result<Vec<OllamaModel>> {
        let resp = self
            .http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Ollama returned: {}", resp.status()));
        }

        let data: OllamaTagResponse = resp.json().await?;
        let models = data.models.into_iter().map(|m| OllamaModel {
            name: m.name.clone(),
            model: m.model,
            size: m.size,
            parameter_size: m.details.parameter_size.clone(),
            format: m.details.format.clone(),
            family: m.details.family.clone(),
            families: m.details.families.clone(),
            quantization_level: m.details.quantization_level.clone(),
        }).collect();

        Ok(models)
    }

    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<OllamaMessage>,
        tools: Option<Vec<Value>>,
        options: Option<HashMap<String, Value>>,
    ) -> anyhow::Result<reqwest::Response> {
        let req = ChatRequest {
            model: model.to_string(),
            messages,
            tools,
            stream: true,
            options,
        };

        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Ollama chat error: {}", resp.status()));
        }

        Ok(resp)
    }

    pub async fn chat_sync(
        &self,
        model: &str,
        messages: Vec<OllamaMessage>,
        tools: Option<Vec<Value>>,
        options: Option<HashMap<String, Value>>,
    ) -> anyhow::Result<String> {
        let req = ChatRequest {
            model: model.to_string(),
            messages,
            tools,
            stream: false,
            options,
        };

        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&req)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama error: {}", text));
        }

        let data: ChatStreamChunk = resp.json().await?;
        Ok(data.message.map(|m| m.content).unwrap_or_default())
    }

    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        system: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut body = serde_json::Map::new();
        body.insert("model".into(), model.into());
        body.insert("prompt".into(), prompt.into());
        body.insert("stream".into(), false.into());
        if let Some(s) = system {
            body.insert("system".into(), s.into());
        }

        let resp = self
            .http
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?;

        let json: Value = resp.json().await?;
        Ok(json.get("response").and_then(|v| v.as_str()).unwrap_or("").to_string())
    }

    pub async fn is_healthy(&self) -> bool {
        self.http
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}
