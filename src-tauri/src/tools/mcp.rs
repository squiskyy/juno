use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<u64>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
}

pub struct McpConnection {
    #[allow(dead_code)]
    config: McpServerConfig,
    child: Option<Child>,
    request_id: Arc<Mutex<u64>>,
    #[allow(dead_code)]
    pending: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<JsonRpcResponse>>>>,
}

impl McpConnection {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config: config.clone(),
            child: None,
            request_id: Arc::new(Mutex::new(0)),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .envs(&self.config.env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("no stdout"))?;
        let _stderr = child.stderr.take();
        let mut stdin = child.stdin.take();
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: 1,
            method: "initialize".into(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "juno", "version": "0.1.0" }
            })),
        };
        let init_json = serde_json::to_string(&init_req)?;
        if let Some(ref mut s) = stdin {
            s.write_all(format!("{}\n", init_json).as_bytes()).await?;
            s.flush().await?;
        }
        if let Some(Ok(line)) = lines.next_line().await? {
            tracing::debug!("MCP init response: {}", line);
        }
        let notif = serde_json::json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} });
        if let Some(ref mut s) = stdin {
            s.write_all(format!("{}\n", notif).as_bytes()).await?;
            s.flush().await?;
        }
        self.child = Some(child);
        Ok(())
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }
}

pub struct McpManager {
    connections: HashMap<String, Arc<Mutex<McpConnection>>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self { connections: HashMap::new() }
    }
    pub fn add_server(&mut self, config: McpServerConfig) {
        let conn = Arc::new(Mutex::new(McpConnection::new(config.clone())));
        self.connections.insert(config.name.clone(), conn);
    }
    pub async fn start_all(&mut self) -> Vec<String> {
        let mut started = Vec::new();
        for (name, conn) in &self.connections {
            let mut guard = conn.lock().await;
            if guard.start().await.is_ok() {
                started.push(name.clone());
            } else {
                tracing::warn!("Failed to start MCP server '{}'", name);
            }
        }
        started
    }
    pub async fn stop_all(&mut self) {
        for (_, conn) in &self.connections {
            let mut guard = conn.lock().await;
            let _ = guard.stop().await;
        }
    }
}
