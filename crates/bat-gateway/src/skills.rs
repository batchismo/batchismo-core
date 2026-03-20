use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use notify::{Watcher, RecommendedWatcher, RecursiveMode};

/// Configuration for a tool defined by a skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    pub name: String,
    pub description: String,
    pub command: String,
    pub args: Option<Vec<String>>,
    pub input_schema: Option<serde_json::Value>,
    pub working_dir: Option<String>,
}

/// Tools configuration file (tools.toml) for a skill.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillToolsConfig {
    #[serde(default)]
    pub tools: Vec<SkillTool>,
}

/// A loaded skill with its metadata and content.
#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub name: String,
    pub path: PathBuf,
    pub content: String,
    pub enabled: bool,
    pub tools: Vec<SkillTool>,
    pub last_modified: std::time::SystemTime,
}

/// Events emitted by the skill system.
#[derive(Debug, Clone)]
pub enum SkillEvent {
    SkillAdded(String),
    SkillUpdated(String),
    SkillRemoved(String),
    SkillEnabled(String),
    SkillDisabled(String),
}

/// The skill manager handles loading, watching, and managing skills.
pub struct SkillManager {
    skills: Arc<RwLock<HashMap<String, Skill>>>,
    workspace_path: PathBuf,
    event_tx: broadcast::Sender<SkillEvent>,
    _watcher: Option<RecommendedWatcher>,
}

impl SkillManager {
    /// Create a new skill manager for the given workspace path.
    pub fn new(workspace_path: PathBuf) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(100);
        
        let mut manager = Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            workspace_path,
            event_tx,
            _watcher: None,
        };

        // Load existing skills
        manager.scan_and_load_skills()?;
        
        // Set up file watcher for hot reload
        manager.setup_file_watcher()?;
        
        Ok(manager)
    }

    /// Get the skills directory path.
    fn skills_dir(&self) -> PathBuf {
        self.workspace_path.join("skills")
    }

    /// Scan the skills directory and load all skills.
    pub fn scan_and_load_skills(&self) -> Result<()> {
        let skills_dir = self.skills_dir();
        
        // Create skills directory if it doesn't exist
        if !skills_dir.exists() {
            std::fs::create_dir_all(&skills_dir)?;
            info!("Created skills directory: {}", skills_dir.display());
        }

        // Clear existing skills
        self.skills.write().unwrap().clear();

        // Scan for skill directories
        if skills_dir.is_dir() {
            for entry in std::fs::read_dir(&skills_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    if let Some(skill_name) = path.file_name().and_then(|n| n.to_str()) {
                        match self.load_skill(skill_name) {
                            Ok(skill) => {
                                info!("Loaded skill: {}", skill.name);
                                self.skills.write().unwrap().insert(skill.name.clone(), skill);
                            }
                            Err(e) => {
                                warn!("Failed to load skill {}: {}", skill_name, e);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single skill from its directory.
    fn load_skill(&self, skill_name: &str) -> Result<Skill> {
        let skill_dir = self.skills_dir().join(skill_name);
        let skill_file = skill_dir.join("SKILL.md");
        
        if !skill_file.exists() {
            anyhow::bail!("SKILL.md not found in {}", skill_dir.display());
        }

        let content = std::fs::read_to_string(&skill_file)
            .with_context(|| format!("Failed to read {}", skill_file.display()))?;

        let last_modified = skill_file.metadata()?.modified()?;

        // Load tools.toml if it exists
        let tools_file = skill_dir.join("tools.toml");
        let tools = if tools_file.exists() {
            let tools_content = std::fs::read_to_string(&tools_file)
                .with_context(|| format!("Failed to read {}", tools_file.display()))?;
            
            let tools_config: SkillToolsConfig = toml::from_str(&tools_content)
                .with_context(|| format!("Failed to parse {}", tools_file.display()))?;
            
            tools_config.tools
        } else {
            Vec::new()
        };

        Ok(Skill {
            name: skill_name.to_string(),
            path: skill_dir,
            content,
            enabled: true, // TODO: Track enabled state in config
            tools,
            last_modified,
        })
    }

    /// Set up file system watcher for hot reload.
    fn setup_file_watcher(&mut self) -> Result<()> {
        let skills_dir = self.skills_dir();
        let skills_dir_clone = skills_dir.clone();
        let skills = Arc::clone(&self.skills);
        let event_tx = self.event_tx.clone();

        let mut watcher = notify::recommended_watcher(move |res| {
            match res {
                Ok(event) => {
                    debug!("File system event: {:?}", event);
                    if let Err(e) = Self::handle_file_event(&skills_dir_clone, &skills, &event_tx, event) {
                        error!("Failed to handle file system event: {}", e);
                    }
                }
                Err(e) => error!("File watcher error: {}", e),
            }
        })?;

        watcher.watch(&skills_dir, RecursiveMode::Recursive)?;
        self._watcher = Some(watcher);
        info!("Set up file watcher for skills directory: {}", skills_dir.display());

        Ok(())
    }

    /// Handle a file system event for hot reloading.
    fn handle_file_event(
        skills_dir: &Path,
        skills: &Arc<RwLock<HashMap<String, Skill>>>,
        event_tx: &broadcast::Sender<SkillEvent>,
        event: notify::Event,
    ) -> Result<()> {
        use notify::EventKind;

        for path in &event.paths {
            // Only handle events in skills subdirectories
            if !path.starts_with(skills_dir) {
                continue;
            }

            let relative_path = path.strip_prefix(skills_dir)?;
            let skill_name = match relative_path.components().next() {
                Some(std::path::Component::Normal(name)) => {
                    name.to_string_lossy().to_string()
                }
                _ => continue,
            };

            let is_skill_file = path.file_name() == Some(std::ffi::OsStr::new("SKILL.md"));
            let is_tools_file = path.file_name() == Some(std::ffi::OsStr::new("tools.toml"));

            if !is_skill_file && !is_tools_file {
                continue;
            }

            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    info!("Reloading skill due to file change: {} ({})", skill_name, path.display());
                    
                    // Reload the skill
                    let manager_temp = SkillManager {
                        skills: Arc::clone(skills),
                        workspace_path: skills_dir.parent().unwrap().to_path_buf(),
                        event_tx: event_tx.clone(),
                        _watcher: None,
                    };
                    
                    match manager_temp.load_skill(&skill_name) {
                        Ok(skill) => {
                            let mut skills_guard = skills.write().unwrap();
                            let is_new = !skills_guard.contains_key(&skill_name);
                            skills_guard.insert(skill.name.clone(), skill);
                            drop(skills_guard);

                            let event = if is_new {
                                SkillEvent::SkillAdded(skill_name)
                            } else {
                                SkillEvent::SkillUpdated(skill_name)
                            };
                            let _ = event_tx.send(event);
                        }
                        Err(e) => {
                            error!("Failed to reload skill {}: {}", skill_name, e);
                        }
                    }
                }
                EventKind::Remove(_) => {
                    if is_skill_file {
                        info!("Removing skill due to file deletion: {}", skill_name);
                        skills.write().unwrap().remove(&skill_name);
                        let _ = event_tx.send(SkillEvent::SkillRemoved(skill_name));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Get all loaded skills.
    pub fn list_skills(&self) -> Vec<Skill> {
        self.skills.read().unwrap().values().cloned().collect()
    }

    /// Get a specific skill by name.
    pub fn get_skill(&self, name: &str) -> Option<Skill> {
        self.skills.read().unwrap().get(name).cloned()
    }

    /// Enable or disable a skill.
    pub fn set_skill_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let mut skills = self.skills.write().unwrap();
        if let Some(skill) = skills.get_mut(name) {
            skill.enabled = enabled;
            drop(skills);

            let event = if enabled {
                SkillEvent::SkillEnabled(name.to_string())
            } else {
                SkillEvent::SkillDisabled(name.to_string())
            };
            let _ = self.event_tx.send(event);

            // TODO: Persist enabled state to config file
            Ok(())
        } else {
            anyhow::bail!("Skill not found: {}", name)
        }
    }

    /// Get all enabled skills.
    pub fn get_enabled_skills(&self) -> Vec<Skill> {
        self.skills
            .read()
            .unwrap()
            .values()
            .filter(|skill| skill.enabled)
            .cloned()
            .collect()
    }

    /// Subscribe to skill events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<SkillEvent> {
        self.event_tx.subscribe()
    }

    /// Build the skills section for the system prompt.
    pub fn build_skills_prompt_section(&self) -> String {
        let enabled_skills = self.get_enabled_skills();
        
        if enabled_skills.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("\n## Available Skills\n\n");
        prompt.push_str("The following skills are available to help guide your responses:\n\n");

        for skill in enabled_skills {
            prompt.push_str(&format!("### {}\n\n", skill.name));
            prompt.push_str(&skill.content);
            prompt.push_str("\n\n");
        }

        prompt
    }

    /// Get all tools defined by skills.
    pub fn get_skill_tools(&self) -> Vec<SkillTool> {
        let mut all_tools = Vec::new();
        
        for skill in self.get_enabled_skills() {
            all_tools.extend(skill.tools);
        }
        
        all_tools
    }
}

/// Create example skills to demonstrate the system.
pub fn create_example_skills(workspace_path: &Path) -> Result<()> {
    let skills_dir = workspace_path.join("skills");
    std::fs::create_dir_all(&skills_dir)?;

    // File Management skill example
    let file_mgmt_dir = skills_dir.join("file-management");
    std::fs::create_dir_all(&file_mgmt_dir)?;

    let file_mgmt_skill = r#"# Skill: File Management

## Purpose
Read, organize, search, and manage files within permitted paths.

## When to Use
- User asks to find, move, rename, or organize files
- User asks what's in a folder
- User asks to clean up or sort files by type/date

## Tools Available
- `fs_read` — read file contents
- `fs_list` — list directory contents  
- `fs_write` — write file to disk
- `web_fetch` — fetch content from URLs

## Constraints
- Never delete files without explicit permission
- Always confirm before moving more than 5 files at once
- Log all file operations for audit trail

## Notes
User prefers files organized by client name, then by date.
"#;

    std::fs::write(file_mgmt_dir.join("SKILL.md"), file_mgmt_skill)?;

    // Research Assistant skill example
    let research_dir = skills_dir.join("research-assistant");
    std::fs::create_dir_all(&research_dir)?;

    let research_skill = r#"# Skill: Research Assistant

## Purpose
Help with research tasks, information gathering, and fact-checking.

## When to Use
- User asks for current information or news
- User needs help with research projects
- User wants to fact-check information
- User asks about recent developments

## Tools Available
- `web_search` — search the web for current information
- `web_fetch` — fetch specific URLs
- `fs_write` — save research findings

## Approach
1. Use web_search for broad information gathering
2. Use web_fetch to read specific sources
3. Synthesize information from multiple sources
4. Always cite sources and indicate recency of information
5. Save important findings to files for future reference

## Notes
Always verify information from multiple sources when possible.
"#;

    std::fs::write(research_dir.join("SKILL.md"), research_skill)?;

    info!("Created example skills in {}", skills_dir.display());
    Ok(())
}