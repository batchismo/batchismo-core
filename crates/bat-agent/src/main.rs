mod agent_loop;
mod llm;
mod policy;
mod tools;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::ClientOptions;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use bat_types::ipc::{AgentToGateway, GatewayToAgent};
use bat_types::message::Message;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,hyper_util=warn,hyper=warn,reqwest=warn,h2=warn,rustls=warn")
    });
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args: Vec<String> = std::env::args().collect();

    let pipe_name = find_arg(&args, "--pipe")
        .ok_or_else(|| anyhow::anyhow!("--pipe <name> argument required"))?;

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY environment variable not set")?;

    tracing::info!("bat-agent starting, connecting to pipe: {}", pipe_name);
    run_agent(&pipe_name, &api_key).await
}

fn find_arg(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}

// ─── Pipe client ─────────────────────────────────────────────────────────────

struct GatewayPipe {
    writer: tokio::io::WriteHalf<tokio::net::windows::named_pipe::NamedPipeClient>,
    reader: BufReader<tokio::io::ReadHalf<tokio::net::windows::named_pipe::NamedPipeClient>>,
}

impl GatewayPipe {
    /// Connect to the gateway's named pipe, retrying briefly on failure.
    async fn connect(pipe_name: &str) -> Result<Self> {
        let client = loop {
            match ClientOptions::new().open(pipe_name) {
                Ok(c) => break c,
                Err(e) => {
                    // Pipe not ready yet — server may still be setting up
                    let code = e.raw_os_error().unwrap_or(0);
                    if code == 231 || code == 2 {
                        // ERROR_PIPE_BUSY or FILE_NOT_FOUND — retry
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    } else {
                        return Err(e).context("Failed to connect to gateway pipe");
                    }
                }
            }
        };
        let (r, w) = tokio::io::split(client);
        Ok(Self {
            writer: w,
            reader: BufReader::new(r),
        })
    }

    async fn send(&mut self, msg: &AgentToGateway) -> Result<()> {
        let json = serde_json::to_string(msg)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn recv(&mut self) -> Result<Option<GatewayToAgent>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let msg = serde_json::from_str(trimmed)
            .with_context(|| format!("Failed to parse gateway message: {trimmed}"))?;
        Ok(Some(msg))
    }
}

// ─── Agent logic ──────────────────────────────────────────────────────────────

async fn run_agent(pipe_name: &str, api_key: &str) -> Result<()> {
    let mut pipe = GatewayPipe::connect(pipe_name).await?;
    tracing::info!("Connected to gateway pipe");

    // Step 1: receive Init
    let init = pipe
        .recv()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipe closed before Init message"))?;

    let (session_id_str, model, system_prompt, history, path_policies, disabled_tools) = match init {
        GatewayToAgent::Init {
            session_id,
            model,
            system_prompt,
            history,
            path_policies,
            disabled_tools,
        } => (session_id, model, system_prompt, history, path_policies, disabled_tools),
        other => anyhow::bail!("Expected Init, got: {:?}", other),
    };

    let session_id: Uuid = session_id_str
        .parse()
        .context("Invalid session_id in Init message")?;

    // Normalize model name (strip optional "anthropic/" prefix)
    let model = model
        .strip_prefix("anthropic/")
        .unwrap_or(&model)
        .to_string();

    tracing::info!(
        "Initialized: session={}, model={}, history={} msgs",
        session_id,
        model,
        history.len()
    );

    // Step 2: receive UserMessage
    let user_content = match pipe.recv().await? {
        Some(GatewayToAgent::UserMessage { content }) => content,
        Some(GatewayToAgent::Cancel) => {
            tracing::info!("Received Cancel before UserMessage — exiting");
            return Ok(());
        }
        None => anyhow::bail!("Pipe closed before UserMessage"),
        other => anyhow::bail!("Expected UserMessage, got: {:?}", other),
    };

    tracing::info!("Running turn for: {:?}", &user_content[..user_content.len().min(80)]);

    // Step 3: create streaming channel for text deltas
    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(128);

    // Build LLM client and tool registry (filtering out any disabled tools)
    let client = llm::AnthropicClient::new(api_key.to_string());
    let registry = tools::ToolRegistry::with_fs_tools(path_policies, &disabled_tools);

    // Step 4: run agent turn in a separate task, streaming text deltas
    let turn_handle = tokio::spawn(async move {
        agent_loop::run_turn_streaming(
            &client,
            &registry,
            &model,
            &system_prompt,
            &history,
            &user_content,
            session_id,
            tx,
        )
        .await
    });

    // Step 5: forward text deltas to the gateway as they arrive
    while let Some(chunk) = rx.recv().await {
        pipe.send(&AgentToGateway::TextDelta { content: chunk })
            .await?;
    }

    // Step 6: get the final turn result
    let turn_result = turn_handle
        .await
        .context("Agent turn task panicked")??;

    // Forward tool events (informational — tools already ran in-process)
    for tc in &turn_result.tool_calls {
        pipe.send(&AgentToGateway::ToolCallStart {
            tool_call: tc.clone(),
        })
        .await?;
    }
    for tr in &turn_result.tool_results {
        pipe.send(&AgentToGateway::ToolCallResult {
            result: tr.clone(),
        })
        .await?;
    }

    // Build and send TurnComplete
    let mut assistant_msg = Message::assistant(session_id, turn_result.response_text);
    assistant_msg.token_input = Some(turn_result.total_input_tokens);
    assistant_msg.token_output = Some(turn_result.total_output_tokens);
    assistant_msg.tool_calls = turn_result.tool_calls;
    assistant_msg.tool_results = turn_result.tool_results;

    pipe.send(&AgentToGateway::TurnComplete {
        message: assistant_msg,
    })
    .await?;

    tracing::info!("Turn complete — agent exiting");
    Ok(())
}
