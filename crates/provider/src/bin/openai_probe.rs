use std::env;

use anyhow::{Context, Result, bail};
use clawcr_provider::openai::{OpenAIProvider, debug_request_body};
use clawcr_provider::{
    ModelProviderSDK, ModelRequest, RequestContent, RequestMessage, ResponseContent, StreamEvent,
    ToolDefinition,
};
use futures::StreamExt;

fn main() -> Result<()> {
    let args = ProbeArgs::from_env()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;
    runtime.block_on(run(args))
}

async fn run(args: ProbeArgs) -> Result<()> {
    let request = ModelRequest {
        model: args.model.clone(),
        system: args.system.clone(),
        messages: vec![RequestMessage {
            role: "user".to_string(),
            content: vec![RequestContent::Text {
                text: args.prompt.clone(),
            }],
        }],
        max_tokens: args.max_tokens,
        tools: args.with_test_tool.then_some(vec![ToolDefinition {
            name: "echo_text".to_string(),
            description: "Return the provided text back to the caller.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"],
                "additionalProperties": false
            }),
        }]),
        temperature: args.temperature,
        thinking: args.thinking.clone(),
    };

    let request_body = debug_request_body(&request, args.stream);
    println!("provider: openai-compatible");
    println!("model: {}", args.model);
    println!("base_url: {}", args.base_url);
    println!("stream: {}", args.stream);
    println!("api_key_present: {}", args.api_key.is_some());
    println!("with_test_tool: {}", args.with_test_tool);
    println!("request_body:");
    println!("{}", serde_json::to_string_pretty(&request_body)?);
    println!();

    let provider = if let Some(api_key) = args.api_key {
        OpenAIProvider::new(args.base_url).with_api_key(api_key)
    } else {
        OpenAIProvider::new(args.base_url)
    };

    if args.stream {
        let mut stream = provider
            .completion_stream(request)
            .await
            .context("start streaming completion")?;
        while let Some(event) = stream.next().await {
            match event.context("stream event failed")? {
                StreamEvent::ContentBlockStart { index, content } => {
                    println!("event: content_block_start index={index} content={content:?}");
                }
                StreamEvent::TextDelta { index, text } => {
                    println!("event: text_delta index={index} text={text:?}");
                }
                StreamEvent::InputJsonDelta {
                    index,
                    partial_json,
                } => {
                    println!("event: input_json_delta index={index} partial_json={partial_json:?}");
                }
                StreamEvent::ContentBlockStop { index } => {
                    println!("event: content_block_stop index={index}");
                }
                StreamEvent::UsageDelta(usage) => {
                    println!(
                        "event: usage_delta input_tokens={} output_tokens={}",
                        usage.input_tokens, usage.output_tokens
                    );
                }
                StreamEvent::MessageDone { response } => {
                    println!("event: message_done");
                    print_response(&response.content);
                    println!(
                        "usage: input_tokens={} output_tokens={}",
                        response.usage.input_tokens, response.usage.output_tokens
                    );
                    println!("stop_reason: {:?}", response.stop_reason);
                }
            }
        }
    } else {
        let response = provider
            .completion(request)
            .await
            .context("run completion")?;
        print_response(&response.content);
        println!(
            "usage: input_tokens={} output_tokens={}",
            response.usage.input_tokens, response.usage.output_tokens
        );
        println!("stop_reason: {:?}", response.stop_reason);
    }

    Ok(())
}

fn print_response(content: &[ResponseContent]) {
    println!("response_content:");
    for block in content {
        match block {
            ResponseContent::Text(text) => {
                println!("  text: {text}");
            }
            ResponseContent::ToolUse { id, name, input } => {
                println!("  tool_use: id={id} name={name} input={input}");
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ProbeArgs {
    model: String,
    base_url: String,
    api_key: Option<String>,
    prompt: String,
    system: Option<String>,
    max_tokens: usize,
    temperature: Option<f64>,
    thinking: Option<String>,
    stream: bool,
    with_test_tool: bool,
}

impl ProbeArgs {
    fn from_env() -> Result<Self> {
        let mut model = env_var("CLAWCR_MODEL");
        let mut base_url = env_var("CLAWCR_BASE_URL").or_else(|| env_var("OPENAI_BASE_URL"));
        let mut api_key = env_var("CLAWCR_API_KEY").or_else(|| env_var("OPENAI_API_KEY"));
        let mut prompt =
            Some(env_var("CLAWCR_PROBE_PROMPT").unwrap_or_else(|| "Reply with FUCK only.".into()));
        let mut system = env_var("CLAWCR_PROBE_SYSTEM");
        let mut max_tokens = Some(256usize);
        let mut temperature = None;
        let mut thinking = env_var("CLAWCR_PROBE_THINKING");
        let mut stream = false;
        let mut with_test_tool = false;

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--model" => model = Some(next_arg(&mut args, "--model")?),
                "--base-url" => base_url = Some(next_arg(&mut args, "--base-url")?),
                "--api-key" => api_key = Some(next_arg(&mut args, "--api-key")?),
                "--prompt" => prompt = Some(next_arg(&mut args, "--prompt")?),
                "--system" => system = Some(next_arg(&mut args, "--system")?),
                "--max-tokens" => {
                    max_tokens = Some(
                        next_arg(&mut args, "--max-tokens")?
                            .parse()
                            .context("parse --max-tokens as usize")?,
                    )
                }
                "--temperature" => {
                    temperature = Some(
                        next_arg(&mut args, "--temperature")?
                            .parse()
                            .context("parse --temperature as f64")?,
                    )
                }
                "--thinking" => thinking = Some(next_arg(&mut args, "--thinking")?),
                "--stream" => stream = true,
                "--with-test-tool" => with_test_tool = true,
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                other => bail!("unknown argument: {other}"),
            }
        }

        let Some(model) = model else {
            print_usage();
            bail!("missing model; pass --model or set CLAWCR_MODEL");
        };
        let Some(base_url) = base_url else {
            print_usage();
            bail!("missing base URL; pass --base-url or set CLAWCR_BASE_URL");
        };

        Ok(Self {
            model,
            base_url,
            api_key,
            prompt: prompt.expect("prompt default is set"),
            system,
            max_tokens: max_tokens.expect("max_tokens default is set"),
            temperature,
            thinking,
            stream,
            with_test_tool,
        })
    }
}

fn env_var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next()
        .with_context(|| format!("missing value for {flag}"))
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!(
        "  cargo run -p clawcr-provider --bin openai_probe -- --model <MODEL> --base-url <URL> [options]"
    );
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --api-key <KEY>");
    eprintln!("  --prompt <TEXT>");
    eprintln!("  --system <TEXT>");
    eprintln!("  --max-tokens <N>");
    eprintln!("  --temperature <F64>");
    eprintln!("  --thinking <VALUE>");
    eprintln!("  --stream");
    eprintln!("  --with-test-tool");
    eprintln!();
    eprintln!("Environment fallbacks:");
    eprintln!("  CLAWCR_MODEL, CLAWCR_BASE_URL, CLAWCR_API_KEY, OPENAI_API_KEY, OPENAI_BASE_URL");
    eprintln!("  CLAWCR_PROBE_PROMPT, CLAWCR_PROBE_SYSTEM, CLAWCR_PROBE_THINKING");
}
