use anyhow::Result;

/// Web search tool that uses OpenAI's Responses API with the `web_search` built-in tool.
pub struct WebSearch {
    api_key: String,
}

impl WebSearch {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

impl super::ToolExecutor for WebSearch {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for current information. Returns relevant search results with titles, URLs, and snippets. Use this when you need up-to-date information that may not be in your training data."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                }
            },
            "required": ["query"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let query = input["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'query' parameter"))?;

        let handle = tokio::runtime::Handle::current();
        let api_key = self.api_key.clone();
        let query = query.to_string();

        handle.block_on(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?;

            let body = serde_json::json!({
                "model": "gpt-4.1-mini",
                "tools": [{ "type": "web_search" }],
                "input": query,
            });

            let resp = client
                .post("https://api.openai.com/v1/responses")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            let status = resp.status();
            let resp_text = resp.text().await?;

            if !status.is_success() {
                anyhow::bail!("OpenAI API error ({}): {}", status.as_u16(), resp_text);
            }

            // Parse the response and extract the text output
            let parsed: serde_json::Value = serde_json::from_str(&resp_text)?;

            // The Responses API returns output items; find the message output
            let mut result_parts: Vec<String> = Vec::new();

            if let Some(output) = parsed["output"].as_array() {
                for item in output {
                    match item["type"].as_str() {
                        Some("message") => {
                            if let Some(content) = item["content"].as_array() {
                                for part in content {
                                    if let Some(text) = part["text"].as_str() {
                                        result_parts.push(text.to_string());
                                    }
                                    // Include annotations (citations) if present
                                    if let Some(annotations) = part["annotations"].as_array() {
                                        for ann in annotations {
                                            if let (Some(title), Some(url)) = (
                                                ann["title"].as_str(),
                                                ann["url"].as_str(),
                                            ) {
                                                result_parts.push(format!("  - [{}]({})", title, url));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if result_parts.is_empty() {
                Ok("No search results found.".to_string())
            } else {
                Ok(result_parts.join("\n"))
            }
        })
    }
}
