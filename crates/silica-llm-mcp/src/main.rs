//! `silica-llm-mcp`: a Model Context Protocol server in front of a local
//! model, so bulk text work can be done off the caller's own token budget.
//!
//! This is the binary a user registers in `.mcp.json`:
//!
//! ```json
//! {"mcpServers": {"silica-llm": {"command": "target/release/silica-llm-mcp"}}}
//! ```
//!
//! The transport is stdio, so **stdout carries the protocol and nothing else**
//! — one JSON object per line. Every log line goes to stderr. The tools are in
//! [`tools`], the framing in [`rpc`], the model in [`model`], and [`http`] is
//! the small HTTP client that reaches it.
//!
//! The server is any OpenAI-compatible endpoint — Ollama, `llama-server`,
//! vLLM, or anything else speaking `/v1/chat/completions`. It is configured
//! by one variable, so moving from one to another is `SILICA_LLM_URL`, not a
//! new code path:
//!
//! ```sh
//! SILICA_LLM_URL=http://127.0.0.1:11434/v1   # Ollama default
//! SILICA_LLM_URL=http://127.0.0.1:8080/v1     # llama-server
//! ```
//!
//! Configuration, all environment variables: `SILICA_LLM_URL`,
//! `SILICA_LLM_MODEL`, `SILICA_LLM_KEY`, `SILICA_LLM_TIMEOUT_MS`,
//! `SILICA_LLM_MAX_BYTES` and `SILICA_LLM_DIGEST_ROOT`.
//!
//! `SILICA_LLM_URL` may be `https://` as well as `http://`; `native-tls` uses
//! the operating system's native trust store, so remote model servers are
//! reached over an encrypted connection. `SILICA_LLM_DIGEST_ROOT` bounds the
//! files `llm_digest` may read (defaults to a `digest` folder next to the
//! server's working directory; there is no unrestricted mode).
//!
//! Two properties of that router shape how these tools behave, and both belong
//! in the code rather than only in a note. It keeps **one** model resident
//! (`--models-max 1`), so naming a preset other than the loaded one evicts it
//! and costs a 10-30 s reload: pick one model for a run. And it sleeps an idle
//! model after 30 minutes, so the first call after a quiet afternoon pays that
//! reload as well — which is why the default timeout is minutes, not seconds.
//! A slow first call is the model waking, not a hang.

mod http;
mod model;
mod rpc;
mod tools;

use std::io::{BufRead, Write};

fn main() {
    let config = match model::Config::from_env() {
        Ok(config) => config,
        Err(e) => {
            // ! A bad SILICA_LLM_URL cannot be reported through the handshake,
            // ! and a server that answers every call with the same error is
            // ! worse than one that refuses to start.
            eprintln!("silica-llm-mcp: SILICA_LLM_URL: {e}");
            std::process::exit(2);
        }
    };
    // * The handshake has no room for this, and a wrong endpoint or model name
    // * is the first thing to go wrong, so say both before answering anything.
    eprintln!(
        "silica-llm-mcp {} -> {} (model: {}, timeout {} ms, digest cap {} KiB)",
        env!("CARGO_PKG_VERSION"),
        config.url.origin(),
        config.model,
        config.timeout.as_millis(),
        config.max_bytes / 1024,
    );
    let backend = model::Backend::new(config);
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    // * Cap line size so a hostile or broken client cannot exhaust memory
    //   with a single unterminated line.
    const MAX_LINE_BYTES: usize = 1 << 20; // 1 MiB
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                eprintln!("silica-llm-mcp: stdin: {e}");
                break;
            }
        };
        if line.len() > MAX_LINE_BYTES {
            eprintln!("silica-llm-mcp: line over {MAX_LINE_BYTES} bytes, dropped");
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Some(reply) = rpc::handle_line(&backend, &line) else {
            continue;
        };
        if writeln!(stdout, "{reply}").is_err() || stdout.flush().is_err() {
            eprintln!("silica-llm-mcp: stdout closed");
            break;
        }
    }
}
