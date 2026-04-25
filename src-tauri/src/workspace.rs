use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

use crate::models::{FolderInfo, WorkspaceFile};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct WorkspaceFolder {
    pub path: String,
    pub included: bool,
}

pub struct WorkspaceManager {
    folders: Vec<WorkspaceFolder>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self { folders: Vec::new() }
    }

    pub fn add_folder(&mut self, path: &str) -> Result<FolderInfo> {
        let p = Path::new(path);
        if !p.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path));
        }
        if !p.is_dir() {
            return Err(anyhow::anyhow!("Path is not a directory: {}", path));
        }

        for f in &self.folders {
            if f.path == path {
                return Err(anyhow::anyhow!("Folder already added: {}", path));
            }
        }

        let count = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count() as u32;

        let name = p.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string();

        self.folders.push(WorkspaceFolder {
            path: path.to_string(),
            included: true,
        });

        Ok(FolderInfo {
            path: path.to_string(),
            name,
            file_count: count,
            included: true,
        })
    }

    pub fn remove_folder(&mut self, path: &str) {
        self.folders.retain(|f| f.path != path);
    }

    pub fn list_folders(&self) -> Vec<FolderInfo> {
        self.folders.iter().map(|f| {
            let p = Path::new(&f.path);
            let name = p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unnamed")
                .to_string();
            let count = WalkDir::new(&f.path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .count() as u32;

            FolderInfo {
                path: f.path.clone(),
                name,
                file_count: count,
                included: f.included,
            }
        }).collect()
    }

    pub fn list_files(&self, folder_path: &str) -> Result<Vec<WorkspaceFile>> {
        let mut files = Vec::new();
        for entry in WalkDir::new(folder_path).max_depth(3) {
            let entry = entry?;
            let path = entry.path();
            let meta = entry.metadata()?;
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();

            files.push(WorkspaceFile {
                path: path.to_string_lossy().to_string(),
                name,
                size: meta.len(),
                is_dir: meta.is_dir(),
                extension: ext,
            });
        }
        Ok(files)
    }

    pub fn get_context_for_chat(&self) -> String {
        let mut context = String::new();
        let mut file_map: HashMap<String, String> = HashMap::new();

        for folder in &self.folders {
            if !folder.included {
                continue;
            }
            for entry in WalkDir::new(&folder.path).max_depth(2) {
                let Ok(entry) = entry else { continue };
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let ext = path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");

                let is_text = matches!(ext, "rs" | "js" | "ts" | "py" | "go" | "c" | "cpp" | "h" | "hpp" | "java" | "kt" | "swift" | "rb" | "php" | "html" | "css" | "scss" | "json" | "yaml" | "yml" | "toml" | "md" | "txt" | "sh" | "ps1" | "bat" | "xml" | "sql" | "ini" | "cfg" | "dockerfile");

                if !is_text {
                    continue;
                }

                let max_size = 50_000;
                let Ok(content) = std::fs::read_to_string(path) else { continue };
                if content.len() > max_size {
                    file_map.insert(
                        path.to_string_lossy().to_string(),
                        format!("[File too large: {} bytes, truncated]\n{}", content.len(), &content[..max_size.min(content.len())]),
                    );
                } else {
                    file_map.insert(path.to_string_lossy().to_string(), content);
                }
            }
        }

        if !file_map.is_empty() {
            context.push_str("## Workspace Context\n\n");
            context.push_str("The following files are available in the workspace:\n\n");
            for (path, content) in &file_map {
                context.push_str(&format!("### {}\n```\n{}\n```\n\n", path, content));
            }
        }

        context
    }

    pub fn search_files(&self, query: &str) -> Vec<String> {
        let mut results = Vec::new();
        for folder in &self.folders {
            if !folder.included {
                continue;
            }
            for entry in WalkDir::new(&folder.path).max_depth(3) {
                let Ok(entry) = entry else { continue };
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(path) else { continue };
                if content.to_lowercase().contains(&query.to_lowercase()) {
                    results.push(path.to_string_lossy().to_string());
                }
            }
        }
        results.dedup();
        results.truncate(20);
        results
    }
}
