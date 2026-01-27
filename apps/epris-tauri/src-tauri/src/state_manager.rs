use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ToolchainVersion {
    pub version: String,
    pub source: String, // "system" or "local"
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ToolchainState {
    pub node: Option<ToolchainVersion>,
    pub pnpm: Option<ToolchainVersion>,
    pub provider_cli: Option<ToolchainVersion>,
    pub remotion_skills: Option<RemotionSkillState>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RemotionSkillState {
    pub source: String,
    pub commit: String,
    pub installed_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct WorkspaceState {
    pub path: String,
    pub provider: String, // "opencode" or "gemini"
    pub last_env_check_at: Option<String>,
    pub skills: Option<RemotionSkillState>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppStateStore {
    pub toolchain: ToolchainState,
    pub workspaces: HashMap<String, WorkspaceState>,
    pub gemini_api_key: Option<String>,
}

pub struct StateManager {
    pub path: PathBuf,
}

impl StateManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            path: app_data_dir.join("state.json"),
        }
    }

    pub fn read(&self) -> AppStateStore {
        if !self.path.exists() {
            return AppStateStore::default();
        }
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn write(&self, state: &AppStateStore) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let content = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
    
    pub fn update<F>(&self, update_fn: F) -> Result<AppStateStore, String> 
    where F: FnOnce(&mut AppStateStore) 
    {
        let mut state = self.read();
        update_fn(&mut state);
        self.write(&state)?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_persistence() {
        // Use a temporary file path
        let tmp_dir = std::env::temp_dir().join("epris_state_test_".to_owned() + &uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&tmp_dir).unwrap();
        
        let mgr = StateManager::new(tmp_dir.clone());
        
        // Read empty
        let initial = mgr.read();
        assert!(initial.workspaces.is_empty());

        // Write
        let mut state = AppStateStore::default();
        state.toolchain.node = Some(ToolchainVersion { version: "18.0.0".into(), source: "test".into() });
        mgr.write(&state).unwrap();

        // Read back
        let loaded = mgr.read();
        assert_eq!(loaded.toolchain.node.unwrap().version, "18.0.0");
        
        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
