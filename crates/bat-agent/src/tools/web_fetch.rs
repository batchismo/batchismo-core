use anyhow::Result;

pub struct WebFetch;

impl WebFetch {
    pub fn new() -> Self {
        Self
    }
}

impl super::ToolExecutor for WebFetch {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch the contents of a URL and return the body as text. Supports HTTP and HTTPS. Returns up to 50KB of content."
    }

    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (must start with http:// or https://)"
                }
            },
            "required": ["url"]
        })
    }

    fn execute(&self, input: &serde_json::Value) -> Result<String> {
        let url = input["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing 'url' parameter"))?;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            anyhow::bail!("URL must start with http:// or https://");
        }

        // Use tokio's current runtime to make the async request
        let handle = tokio::runtime::Handle::current();
        let url_owned = url.to_string();

        let result = handle.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("Batchismo/0.1")
                .build()?;

            let resp = client.get(&url_owned).send().await?;
            let status = resp.status();

            if !status.is_success() {
                anyhow::bail!("HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));
            }

            let body = resp.text().await?;
            Ok::<String, anyhow::Error>(body)
        })?;

        // Truncate to 50KB
        if result.len() > 50_000 {
            Ok(format!(
                "{}\n\n[Truncated: response is {} bytes, showing first 50,000 characters]",
                &result[..50_000],
                result.len()
            ))
        } else {
            Ok(result)
        }
    }
}
