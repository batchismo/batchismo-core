use anyhow::{bail, Result};
use bat_types::policy::{check_access, strip_win_prefix, PathPolicy};
use std::path::Path;

use crate::llm::{AnthropicClient, AnthropicMessage, ChatRequest};

pub struct FsReadPdf {
    policies: Vec<PathPolicy>,
    api_key: String,
}

impl FsReadPdf {
    pub fn new(policies: Vec<PathPolicy>, api_key: String) -> Self {
        Self { policies, api_key }
    }
}

impl super::ToolExecutor for FsReadPdf {
    fn name(&self) -> &str {
        "fs_read_pdf"
    }

    fn description(&self) -> &str {
        "Read a PDF file and extract its text content using Claude. Returns the extracted text."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the PDF file to read"
                }
            },
            "required": ["path"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let path_str = input["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'path' parameter"))?;

        let path = Path::new(path_str)
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("Cannot resolve path '{}': {}", path_str, e))?;

        if !check_access(&self.policies, &path, false) {
            bail!(
                "Access denied: '{}' is not in any allowed read policy",
                strip_win_prefix(&path).display()
            );
        }

        // Read PDF bytes
        let bytes = std::fs::read(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", strip_win_prefix(&path).display(), e))?;

        // Enforce a size limit (32MB)
        if bytes.len() > 32 * 1024 * 1024 {
            bail!("PDF file is too large ({} bytes, max 32MB)", bytes.len());
        }

        // Base64 encode
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);

        // Call Anthropic API with document content block
        let client = AnthropicClient::new(self.api_key.clone());
        let request = ChatRequest {
            model: "claude-haiku-4-5-latest".to_string(),
            max_tokens: 8192,
            system: "You are a document text extractor. Extract ALL text content from the provided PDF document. Return ONLY the extracted text, preserving the original structure (headings, paragraphs, lists) as much as possible. Do not add commentary or summaries.".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: serde_json::json!([
                    {
                        "type": "document",
                        "source": {
                            "type": "base64",
                            "media_type": "application/pdf",
                            "data": encoded
                        }
                    },
                    {
                        "type": "text",
                        "text": "Extract all text content from this PDF document."
                    }
                ]),
            }],
            tools: vec![],
            stream: false,
        };

        // Block on async call from sync context
        let handle = tokio::runtime::Handle::current();
        let response = handle.block_on(client.chat(&request))?;

        let text = response.text();
        if text.is_empty() {
            bail!("No text could be extracted from the PDF");
        }

        Ok(text)
    }
}
