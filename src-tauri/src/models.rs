use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub model: String,
    pub size: u64,
    pub parameter_size: String,
    pub format: String,
    pub family: String,
    pub families: Option<Vec<String>>,
    pub quantization_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderInfo {
    pub path: String,
    pub name: String,
    pub file_count: u32,
    pub included: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub enabled: bool,
    pub available: bool,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_info_serde() {
        let f = FolderInfo {
            path: "/tmp/test".into(),
            name: "test".into(),
            file_count: 42,
            included: true,
        };
        let json = serde_json::to_string(&f).unwrap();
        let back: FolderInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test");
        assert_eq!(back.file_count, 42);
    }
}
