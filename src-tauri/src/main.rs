use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

mod chat;
mod models;
mod ollama;
mod tools;
mod workspace;

use chat::{ChatEngine, ChatMessage};
use models::*;
use ollama::OllamaClient;
use tools::ToolRegistry;
use workspace::WorkspaceManager;

pub struct AppState {
    pub ollama: Arc<Mutex<OllamaClient>>,
    pub engine: Arc<Mutex<ChatEngine>>,
    pub workspace: Arc<Mutex<WorkspaceManager>>,
    pub tools: Arc<Mutex<ToolRegistry>>,
}

#[tauri::command]
async fn get_models(state: State<'_, AppState>) -> Result<Vec<OllamaModel>, String> {
    state.ollama.lock().await.list_models().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat(message: String, model: String, state: State<'_, AppState>) -> Result<String, String> {
    let mut engine = state.engine.lock().await;
    let tools = state.tools.lock().await;
    let workspace = state.workspace.lock().await;
    engine.chat(&model, &message, &tools, &workspace).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_folder(path: String, state: State<'_, AppState>) -> Result<FolderInfo, String> {
    state.workspace.lock().await.add_folder(&path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_folder(path: String, state: State<'_, AppState>) -> Result<(), String> {
    state.workspace.lock().await.remove_folder(&path);
    Ok(())
}

#[tauri::command]
async fn list_folders(state: State<'_, AppState>) -> Result<Vec<FolderInfo>, String> {
    Ok(state.workspace.lock().await.list_folders())
}

#[tauri::command]
async fn get_chat_history(state: State<'_, AppState>) -> Result<Vec<ChatMessage>, String> {
    Ok(state.engine.lock().await.history.clone())
}

#[tauri::command]
async fn clear_chat(state: State<'_, AppState>) -> Result<(), String> {
    state.engine.lock().await.clear();
    Ok(())
}

#[tauri::command]
async fn get_tool_status(state: State<'_, AppState>) -> Result<Vec<ToolStatus>, String> {
    Ok(state.tools.lock().await.status())
}

#[tauri::command]
async fn toggle_tool(name: String, enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.tools.lock().await.set_enabled(&name, enabled);
    Ok(())
}

#[tauri::command]
async fn get_workspace_files(folder_path: String, state: State<'_, AppState>) -> Result<Vec<WorkspaceFile>, String> {
    state.workspace.lock().await.list_files(&folder_path).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ollama = Arc::new(Mutex::new(OllamaClient::new("http://localhost:11434")));
    let engine = Arc::new(Mutex::new(ChatEngine::new()));
    let workspace = Arc::new(Mutex::new(WorkspaceManager::new()));
    let tools = Arc::new(Mutex::new(ToolRegistry::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { ollama, engine, workspace, tools })
        .invoke_handler(tauri::generate_handler![
            get_models, chat, add_folder, remove_folder, list_folders,
            get_chat_history, clear_chat, get_tool_status, toggle_tool,
            get_workspace_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running juno");
}

fn main() { run() }
