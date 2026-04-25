use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub enabled: bool,
    pub available: bool,
    pub schema: Value,
    pub handler: Box<dyn Fn(&Value) -> String + Send + Sync>,
}

pub fn make_schema(name: &str, description: &str, params: &[(&str, &str, &str, bool)]) -> Value {
    use serde_json::Map;
    let mut properties = Map::new();
    let mut required = Vec::new();
    for (p_name, p_type, p_desc, req) in params {
        let mut prop = Map::new();
        prop.insert("type".into(), Value::String((*p_type).into()));
        prop.insert("description".into(), Value::String((*p_desc).into()));
        properties.insert((*p_name).into(), Value::Object(prop));
        if *req { required.push(Value::String((*p_name).into())); }
    }
    let mut fn_map = Map::new();
    fn_map.insert("name".into(), Value::String(name.into()));
    fn_map.insert("description".into(), Value::String(description.into()));
    let mut params_map = Map::new();
    params_map.insert("type".into(), Value::String("object".into()));
    params_map.insert("properties".into(), Value::Object(properties));
    params_map.insert("required".into(), Value::Array(required));
    fn_map.insert("parameters".into(), Value::Object(params_map));
    let mut schema = Map::new();
    schema.insert("type".into(), Value::String("function".into()));
    schema.insert("function".into(), Value::Object(fn_map));
    Value::Object(schema)
}

fn read_file_tool(args: &Value) -> String {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    if path.is_empty() { return "Error: no path provided".into(); }
    match std::fs::read_to_string(path) {
        Ok(c) => format!("Contents of {}:\n```\n{}\n```", path, c),
        Err(e) => format!("Error reading {}: {}", path, e),
    }
}

fn write_file_tool(args: &Value) -> String {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
    if path.is_empty() { return "Error: no path provided".into(); }
    match std::fs::write(path, content) {
        Ok(_) => format!("Wrote {} bytes to {}", content.len(), path),
        Err(e) => format!("Error writing {}: {}", path, e),
    }
}

fn list_dir_tool(args: &Value) -> String {
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let Ok(entries) = std::fs::read_dir(path) else {
        return format!("Error: could not list directory {}", path);
    };
    let mut lines = vec![format!("Directory listing for {}:", path)];
    for entry in entries {
        let Ok(e) = entry else { continue; };
        let name = e.file_name().to_string_lossy().to_string();
        let prefix = match e.metadata() {
            Ok(m) => if m.is_dir() { "[DIR]" } else { "[FILE]" },
            Err(_) => "[?]",
        };
        lines.push(format!("{} {}", prefix, name));
    }
    lines.join("\n")
}

fn search_files_tool(args: &Value) -> String {
    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
    let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    if query.is_empty() { return "Error: no query provided".into(); }
    let mut results = Vec::new();
    for entry in walkdir::WalkDir::new(path).max_depth(5) {
        let Ok(entry) = entry else { continue; };
        let p = entry.path();
        if !p.is_file() { continue; }
        let Ok(content) = std::fs::read_to_string(p) else { continue; };
        if content.to_lowercase().contains(&query.to_lowercase()) {
            results.push(p.to_string_lossy().to_string());
            if results.len() >= 10 { break; }
        }
    }
    if results.is_empty() {
        format!("No files found matching '{}' in {}", query, path)
    } else {
        format!("Found {} matches:\n{}", results.len(), results.join("\n"))
    }
}

fn shell_tool(args: &Value) -> String {
    let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
    if cmd.is_empty() { return "Error: no command provided".into(); }

    #[cfg(target_os = "windows")]
    let out = std::process::Command::new("cmd").args(["/C", cmd]).output();
    #[cfg(not(target_os = "windows"))]
    let out = std::process::Command::new("sh").args(["-c", cmd]).output();

    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if !stderr.is_empty() {
                format!("stdout:\n{}\nstderr:\n{}", stdout, stderr)
            } else {
                stdout.to_string()
            }
        }
        Err(e) => format!("Error executing command: {}", e),
    }
}

fn get_env_tool(args: &Value) -> String {
    let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
    if name.is_empty() { return "Error: no variable name provided".into(); }
    std::env::var(name).unwrap_or_else(|_| "(not set)".into())
}

fn computer_screenshot_tool(_: &Value) -> String {
    #[cfg(target_os = "windows")]
    match computer_use::capture_screenshot() {
        Ok(p) => format!("Screenshot saved to: {}", p.display()),
        Err(e) => format!("Screenshot failed: {}", e),
    }
    #[cfg(not(target_os = "windows"))]
    "Screenshot tool is only available on Windows".into()
}

fn computer_click_tool(args: &Value) -> String {
    let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let btn = args.get("button").and_then(|v| v.as_str()).unwrap_or("left");
    #[cfg(target_os = "windows")]
    {
        let b = computer_use::MouseButton::from_str(btn);
        match computer_use::click(x, y, b) {
            Ok(_) => format!("Clicked {} button at ({}, {})", btn, x, y),
            Err(e) => format!("Click failed: {}", e),
        }
    }
    #[cfg(not(target_os = "windows"))]
    { format!("Computer use tools only available on Windows (requested click at {}, {})", x, y) }
}

fn computer_type_tool(args: &Value) -> String {
    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() { return "Error: no text provided".into(); }
    #[cfg(target_os = "windows")]
    match computer_use::type_text(text) {
        Ok(_) => format!("Typed: {}", text),
        Err(e) => format!("Type failed: {}", e),
    }
    #[cfg(not(target_os = "windows"))]
    { format!("Computer use tools only available on Windows (would type: {})", text) }
}

pub struct ToolRegistry {
    tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut m = HashMap::new();

        m.insert("read_file".into(), Tool {
            name: "read_file".into(), display_name: "Read File".into(),
            description: "Read the contents of a file".into(),
            enabled: true, available: true,
            schema: make_schema("read_file", "Read file contents", &[("path", "string", "File path", true)]),
            handler: Box::new(read_file_tool),
        });
        m.insert("write_file".into(), Tool {
            name: "write_file".into(), display_name: "Write File".into(),
            description: "Write content to a file".into(),
            enabled: true, available: true,
            schema: make_schema("write_file", "Write file contents", &[("path","string","File path",true),("content","string","Content",true)]),
            handler: Box::new(write_file_tool),
        });
        m.insert("list_directory".into(), Tool {
            name: "list_directory".into(), display_name: "List Directory".into(),
            description: "List files in a directory".into(),
            enabled: true, available: true,
            schema: make_schema("list_directory", "List directory", &[("path","string","Directory path",false)]),
            handler: Box::new(list_dir_tool),
        });
        m.insert("search_files".into(), Tool {
            name: "search_files".into(), display_name: "Search Files".into(),
            description: "Search for text in files".into(),
            enabled: true, available: true,
            schema: make_schema("search_files", "Search files", &[("query","string","Search text",true),("path","string","Directory",false)]),
            handler: Box::new(search_files_tool),
        });
        m.insert("shell".into(), Tool {
            name: "shell".into(), display_name: "Shell Command".into(),
            description: "Run a shell command".into(),
            enabled: true, available: true,
            schema: make_schema("shell", "Shell command", &[("command","string","Command",true)]),
            handler: Box::new(shell_tool),
        });
        m.insert("get_env".into(), Tool {
            name: "get_env".into(), display_name: "Get Env Var".into(),
            description: "Get environment variable".into(),
            enabled: false, available: true,
            schema: make_schema("get_env", "Get env var", &[("name","string","Variable name",true)]),
            handler: Box::new(get_env_tool),
        });
        m.insert("computer_screenshot".into(), Tool {
            name: "computer_screenshot".into(), display_name: "Screenshot".into(),
            description: "Take a screenshot".into(),
            enabled: false, available: cfg!(target_os = "windows"),
            schema: make_schema("computer_screenshot", "Screenshot", &[]),
            handler: Box::new(computer_screenshot_tool),
        });
        m.insert("computer_click".into(), Tool {
            name: "computer_click".into(), display_name: "Mouse Click".into(),
            description: "Click at screen coordinates".into(),
            enabled: false, available: cfg!(target_os = "windows"),
            schema: make_schema("computer_click", "Click at position", &[
                ("x","integer","X coord",true),("y","integer","Y coord",true),("button","string","Button",false)
            ]),
            handler: Box::new(computer_click_tool),
        });
        m.insert("computer_type".into(), Tool {
            name: "computer_type".into(), display_name: "Type Text".into(),
            description: "Type text at cursor position".into(),
            enabled: false, available: cfg!(target_os = "windows"),
            schema: make_schema("computer_type", "Type text", &[("text","string","Text to type",true)]),
            handler: Box::new(computer_type_tool),
        });

        Self { tools: m }
    }

    pub fn enabled_schemas(&self) -> Vec<Value> {
        self.tools.values().filter(|t| t.enabled && t.available).map(|t| t.schema.clone()).collect()
    }

    pub fn status(&self) -> Vec<crate::models::ToolStatus> {
        self.tools.values().map(|t| crate::models::ToolStatus {
            name: t.name.clone(),
            display_name: t.display_name.clone(),
            description: t.description.clone(),
            enabled: t.enabled,
            available: t.available,
        }).collect()
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) {
        if let Some(t) = self.tools.get_mut(name) { t.enabled = enabled; }
    }

    pub async fn execute(&self, name: &str, args: &Value) -> String {
        if let Some(t) = self.tools.get(name) {
            if !t.enabled { return format!("Tool '{}' is disabled", name); }
            (t.handler)(args)
        } else {
            format!("Unknown tool: {}", name)
        }
    }
}

pub mod computer_use;
pub mod mcp;
