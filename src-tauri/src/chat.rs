use serde::{Deserialize, Serialize};
use crate::ollama::{OllamaClient, OllamaMessage};
use crate::tools::{ToolCall, ToolRegistry};
use crate::workspace::WorkspaceManager;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

pub struct ChatEngine {
    pub history: Vec<ChatMessage>,
    pub system_prompt: String,
}

impl ChatEngine {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            system_prompt: Self::default_system_prompt(),
        }
    }

    pub fn default_system_prompt() -> String {
        "You are Juno, a helpful AI agent running locally via Ollama. \
You have access to tools for reading files, writing files, listing directories, \
searching files, running shell commands, and on Windows: taking screenshots, \
clicking, and typing. Use these tools when the user references workspace files \
or asks you to perform actions. Keep responses clear and concise.".to_string()
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }

    pub async fn chat(
        &mut self,
        model: &str,
        user_message: &str,
        tools: &ToolRegistry,
        workspace: &WorkspaceManager,
    ) -> Result<String> {
        self.history.push(ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: "user".to_string(),
            content: user_message.to_string(),
            timestamp: chrono::Local::now().to_rfc3339(),
            tool_calls: None,
        });

        let context = workspace.get_context_for_chat();
        let ollama = OllamaClient::new("http://localhost:11434");

        let mut messages: Vec<OllamaMessage> = vec![OllamaMessage {
            role: "system".to_string(),
            content: if context.is_empty() {
                self.system_prompt.clone()
            } else {
                format!("{}\n\n## Workspace Context\n\n{}", self.system_prompt, context)
            },
            tool_calls: None,
            images: None,
        }];

        for msg in &self.history {
            messages.push(OllamaMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
                tool_calls: msg.tool_calls.as_ref().map(|tc|
                    tc.iter().map(|t| serde_json::to_value(t).unwrap_or(serde_json::Value::Null)).collect()
                ),
                images: None,
            });
        }

        let schemas = tools.enabled_schemas();
        let response: String;

        if schemas.is_empty() {
            response = ollama.chat_sync(model, messages, None, None).await?;
        } else {
            let mut result = ollama.chat_sync(model, messages.clone(), Some(schemas), None).await?;

            if let Ok(tc) = serde_json::from_str::<ToolCall>(&result) {
                if !tc.name.is_empty() {
                    let exec = tools.execute(&tc.name, &tc.arguments).await;
                    messages.push(OllamaMessage {
                        role: "tool".to_string(),
                        content: exec.clone(),
                        tool_calls: None,
                        images: None,
                    });
                    self.history.push(ChatMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        role: "tool".to_string(),
                        content: exec.clone(),
                        timestamp: chrono::Local::now().to_rfc3339(),
                        tool_calls: Some(vec![tc]),
                    });
                    result = ollama.chat_sync(model, messages, None, None).await?;
                }
            }
            response = result;
        }

        self.history.push(ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: "assistant".to_string(),
            content: response.clone(),
            timestamp: chrono::Local::now().to_rfc3339(),
            tool_calls: None,
        });

        Ok(response)
    }
}
