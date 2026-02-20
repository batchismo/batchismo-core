use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use bat_types::config::BatConfig;

/// Returns the Batchismo home directory (~/.batchismo/)
pub fn bat_home() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".batchismo")
}

/// Returns the path to the config file (~/.batchismo/config.toml)
pub fn config_path() -> PathBuf {
    bat_home().join("config.toml")
}

/// Returns the database path (~/.batchismo/batchismo.db)
pub fn db_path() -> PathBuf {
    bat_home().join("batchismo.db")
}

/// Returns the workspace path (~/.batchismo/workspace/)
pub fn workspace_path() -> PathBuf {
    bat_home().join("workspace")
}

/// Load config from disk, creating default if it doesn't exist.
pub fn load_config() -> Result<BatConfig> {
    let path = config_path();

    if !path.exists() {
        // Create ~/.batchismo/ and write default config
        let home = bat_home();
        std::fs::create_dir_all(&home)
            .with_context(|| format!("Failed to create {}", home.display()))?;

        let default = BatConfig::default();
        let toml_str = toml::to_string_pretty(&default)
            .context("Failed to serialize default config")?;
        std::fs::write(&path, &toml_str)
            .with_context(|| format!("Failed to write default config to {}", path.display()))?;

        // Also create workspace directory with default MD files
        init_workspace(&workspace_path())?;

        return Ok(default);
    }

    let contents = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;
    let config: BatConfig = toml::from_str(&contents)
        .with_context(|| format!("Failed to parse config at {}", path.display()))?;
    Ok(config)
}

/// Save config to disk, overwriting the existing file.
pub fn save_config(config: &BatConfig) -> Result<()> {
    let path = config_path();
    let toml_str = toml::to_string_pretty(config)
        .context("Failed to serialize config")?;
    std::fs::write(&path, toml_str)
        .with_context(|| format!("Failed to write config to {}", path.display()))?;
    Ok(())
}

/// Initialize the workspace with default MD files.
fn init_workspace(workspace: &Path) -> Result<()> {
    std::fs::create_dir_all(workspace)?;

    let identity = workspace.join("IDENTITY.md");
    if !identity.exists() {
        std::fs::write(&identity, "# Agent Identity\n\n## Name\nAria\n\n## Role\nPersonal AI assistant.\n")?;
    }

    let memory = workspace.join("MEMORY.md");
    if !memory.exists() {
        std::fs::write(&memory, "# User Memory\n\n_No memories yet. I'll learn about you over time._\n")?;
    }

    let skills = workspace.join("SKILLS.md");
    if !skills.exists() {
        std::fs::write(&skills, "# Skills Index\n\n- **File Management** — Read, write, and organize files\n")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bat_home_exists() {
        let home = bat_home();
        assert!(home.to_string_lossy().contains(".batchismo"));
    }

    #[test]
    fn default_config_roundtrips() {
        let config = BatConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: BatConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.agent.name, "Aria");
    }
}
