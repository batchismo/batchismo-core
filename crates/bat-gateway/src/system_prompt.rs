/// Build the system prompt from workspace MD files and config.

use anyhow::Result;
use std::path::Path;

use bat_types::config::BatConfig;
use bat_types::policy::PathPolicy;

use crate::config::workspace_path;

/// Read a markdown file, returning an empty string on missing file.
fn read_md(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Format path policies for inclusion in the system prompt.
fn format_policies(policies: &[PathPolicy]) -> String {
    if policies.is_empty() {
        return "  (none configured — all file access will be denied)".to_string();
    }
    policies
        .iter()
        .map(|p| {
            let access = match p.access {
                bat_types::policy::AccessLevel::ReadOnly => "read-only",
                bat_types::policy::AccessLevel::ReadWrite => "read-write",
                bat_types::policy::AccessLevel::WriteOnly => "write-only",
            };
            let scope = if p.recursive { "recursive" } else { "top-level only" };
            format!("  - {} [{}] ({})", p.path.display(), access, scope)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build the full system prompt from config + workspace MD files.
pub fn build_system_prompt(config: &BatConfig, path_policies: &[PathPolicy]) -> Result<String> {
    let workspace = workspace_path();

    let identity = read_md(&workspace.join("IDENTITY.md"));
    let memory = read_md(&workspace.join("MEMORY.md"));
    let skills = read_md(&workspace.join("SKILLS.md"));

    let agent_name = &config.agent.name;
    let policies_str = format_policies(path_policies);

    let prompt = format!(
        r#"You are {agent_name}, a personal AI assistant running on the user's computer via Batchismo.

{identity}

## Capabilities

You have access to the following tools for working with files on the user's computer:
- **fs.read** — Read the contents of a file
- **fs.write** — Write or create files
- **fs.list** — List directory contents

You may only access files within the user's permitted paths. If a path is not in the list below, you will receive an access-denied error.

## Permitted Paths

{policies_str}

## Memory

{memory}

## Skills

{skills}

## Guidelines

- Be helpful, concise, and accurate.
- Always explain what you are about to do before taking file actions.
- If a file operation fails, report the error clearly and suggest alternatives.
- Do not attempt to access paths outside the permitted list.
"#
    );

    Ok(prompt)
}
