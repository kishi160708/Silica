//! The tool table: what a model may ask the local model to do.
//!
//! Three tools, and the split between them is about whose context pays. The
//! caller's own context pays for `llm_generate`, because the prompt and the
//! answer both pass through it. It does not pay for `llm_digest`: the file is
//! read here, inside this process, and only the summary crosses back. That is
//! the whole point of the tool — a 40,000-line `cargo test` log can be reduced
//! to a paragraph without ever entering the conversation.

use crate::model::{Answer, Ask, Backend, digest_path};
use crate::rpc::{Fault, INVALID_PARAMS};
use serde_json::{json, Map, Value};
#[cfg(unix)]
use std::path::Path;
use std::io::{Read, Seek};
#[cfg(unix)]
use std::ffi::CString;
#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};

enum Run {
    Models,
    Generate,
    Digest,
}

struct Tool {
    name: &'static str,
    description: &'static str,
    schema: fn() -> Value,
    run: Run,
}

const TOOLS: &[Tool] = &[
    Tool {
        name: "llm_models",
        description: "\
List the presets the local llama-server router is serving, and say where it \
is. Call this first, and whenever a generate fails: it is what separates \
`nothing is listening on that port` from `no such preset`. The router keeps \
one model resident (`--models-max 1`) and sleeps it after 30 minutes idle, so \
naming a different preset than the resident one costs a 10-30 s reload and \
evicts the other. Takes no arguments.",
        schema: || json!({"type": "object", "properties": {}}),
        run: Run::Models,
    },
    Tool {
        name: "llm_generate",
        description: "\
Send one prompt to the local model and return its answer. Use it for bulk, \
low-judgment text work that would otherwise cost the caller's own tokens: \
generating candidate test fixtures, drafting many variations of a string, \
classifying a list, rewriting prose. Do NOT use it for anything that must be \
exact — a command id, a config key, a file path, an API signature — because a \
small local model will produce a plausible wrong string and nothing in the \
answer will mark it as a guess. The answer is prefixed with a one-line \
provenance note giving the model, the token counts and the rate. A \
`<think>` block is stripped before the answer is returned.",
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "prompt": {"type": "string",
                        "description": "The instruction, with everything the model needs: it sees no other context."},
                    "system": {"type": "string",
                        "description": "System message; sent only when given."},
                    "model": {"type": "string",
                        "description": "Override SILICA_LLM_MODEL for this one call. Use llm_models for the names."},
                    "temperature": {"type": "number",
                        "description": "0 for a deterministic answer, 0.8-1.0 when you want variety (generating fixtures). The server's default when omitted."},
                    "max_tokens": {"type": "number",
                        "description": "Cap the completion. Worth setting: an unbounded local generation can run for minutes."},
                    "json": {"type": "boolean",
                        "description": "Ask the server for a single JSON object. llama-server enforces this with a grammar, so the shape is guaranteed."},
                    "enable_thinking": {"type": "boolean",
                        "description": "Qwen's thinking mode. The `-code` presets have it off already; pass false for `qwen3.8-27b` or `qwen3.6-35b-a3b`, which think by default and will otherwise spend most of the budget on it."},
                },
                "required": ["prompt"],
            })
        },
        run: Run::Generate,
    },
    Tool {
        name: "llm_digest",
        description: "\
Read a file here and return only the local model's summary of it, so the file \
itself never enters the caller's context. This is the tool to reach for when \
something has produced far more output than is worth reading: a `cargo test` \
failure wall, a clippy flood, a corpus diff, an Obsidian probe's raw JSON. \
`instruction` says what to pull out — be specific, because a small model \
summarizing without a question gives back a vague paragraph. `contains` \
pre-filters to matching lines before the model sees anything, which is often \
what makes a large log fit at all; prefer it over asking the model to ignore \
the noise. Remember that a summary can drop the one line that mattered: when \
the answer names a specific failure, read that part of the file directly.",
        schema: || {
            json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string",
                        "description": "File to read, absolute or relative to this server's working directory."},
                    "instruction": {"type": "string",
                        "description": "What to extract, e.g. `list each distinct test failure once, with the assertion and the file:line`."},
                    "contains": {"type": "string",
                        "description": "Keep only lines containing this substring (case-insensitive) before the model sees the file."},
                    "tail": {"type": "boolean",
                        "description": "When the file is larger than the byte cap, keep the end rather than the beginning. True for a log whose failure summary is last."},
                    "model": {"type": "string", "description": "Override SILICA_LLM_MODEL for this call."},
                    "max_tokens": {"type": "number", "description": "Cap the summary's length."},
                },
                "required": ["path", "instruction"],
            })
        },
        run: Run::Digest,
    },
];

pub fn list() -> Value {
    let tools: Vec<Value> = TOOLS
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": (tool.schema)(),
            })
        })
        .collect();
    json!({ "tools": tools })
}

/// Run one `tools/call`. The `Err` arm is for a malformed request only: a
/// model server that refused is an `isError` result the caller can read.
pub fn call(backend: &Backend, params: &Value) -> Result<Value, Fault> {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err(Fault::new(INVALID_PARAMS, "tools/call needs a `name`"));
    };
    let Some(tool) = TOOLS.iter().find(|tool| tool.name == name) else {
        return Err(Fault::new(INVALID_PARAMS, format!("unknown tool: {name}")));
    };
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => Value::Object(Map::new()),
        Some(arguments) if arguments.is_object() => arguments.clone(),
        Some(_) => return Err(Fault::new(INVALID_PARAMS, "`arguments` must be an object")),
    };
    Ok(match run(backend, tool, &arguments) {
        Ok(text_out) => json!({"content": [text(text_out)], "isError": false}),
        Err(message) => json!({"content": [text(message)], "isError": true}),
    })
}

fn run(backend: &Backend, tool: &Tool, arguments: &Value) -> Result<String, String> {
    match tool.run {
        Run::Models => {
            let names = backend.models()?;
            let origin = backend.config.url.origin();
            if names.is_empty() {
                return Ok(format!(
                "{origin} is up but serving no models; check \
                     that the model is pulled and the server is serving it (wanted `{}`).",
                    backend.config.model
                ));
            }
            Ok(format!(
                "{origin}, default `{}`{}\n{}",
                backend.config.model,
                if names.contains(&backend.config.model) {
                    String::new()
                } else {
                    format!(
                        " (WARNING: `{}` is not in this list; every call will fail \
                         until SILICA_LLM_MODEL names one that is)",
                        backend.config.model
                    )
                },
                names
                    .iter()
                    .map(|name| format!("  {name}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ))
        }
        Run::Generate => {
            let prompt = required_str(arguments, "prompt")?;
            let answer = backend.chat(&Ask {
                prompt,
                system: str_arg(arguments, "system"),
                model: str_arg(arguments, "model"),
                temperature: arguments.get("temperature").and_then(Value::as_f64),
                max_tokens: arguments.get("max_tokens").and_then(Value::as_u64),
                json: arguments
                    .get("json")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                enable_thinking: arguments.get("enable_thinking").and_then(Value::as_bool),
            })?;
            Ok(with_provenance(&answer))
        }
        Run::Digest => {
            let path = required_str(arguments, "path")?;
            let instruction = required_str(arguments, "instruction")?;
            let excerpt = read_excerpt(
                backend,
                path,
                str_arg(arguments, "contains"),
                arguments
                    .get("tail")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            )?;
            let prompt = format!(
                "{instruction}\n\nBelow is the content of `{path}`{}. Answer only \
                 from it; say so if it does not contain what was asked for. Quote \
                 identifiers, paths and numbers exactly as they appear — never \
                 reconstruct one from memory.\n\n----- begin {path} -----\n{}\n\
                 ----- end {path} -----",
                excerpt.note, excerpt.body,
            );
            let answer = backend.chat(&Ask {
                prompt: &prompt,
                system: Some(
                    "You summarize developer output. Be terse and concrete. \
                     Prefer a list of distinct findings over prose. Copy every \
                     identifier, path and number verbatim.",
                ),
                model: str_arg(arguments, "model"),
                temperature: Some(0.0),
                max_tokens: arguments.get("max_tokens").and_then(Value::as_u64),
                json: false,
                // * A digest wants the answer, not the deliberation, and the
                // * general presets think by default.
                enable_thinking: Some(false),
            })?;
            Ok(format!(
                "{}\n{}",
                excerpt.note.trim(),
                with_provenance(&answer)
            ))
        }
    }
}

fn with_provenance(answer: &Answer) -> String {
    if answer.text.is_empty() {
        return format!(
            "{}\nThe model returned nothing. If the whole budget went to a \
             reasoning block, raise max_tokens or pick a model without one.",
            answer.provenance()
        );
    }
    format!("{}\n{}", answer.provenance(), answer.text)
}

#[derive(Debug)]
struct Excerpt {
    body: String,
    /// What was done to the file, for the caller: a summary of a filtered,
    /// truncated file must never look like a summary of the whole thing.
    note: String,
}

/// Process one complete line (trailing newline already stripped) for the
/// digest excerpt. Returns false when the byte cap is reached and reading
/// must stop.
fn keep_line(
    needle: &Option<String>,
    kept: &mut Vec<String>,
    kept_bytes: &mut usize,
    max: usize,
    line: &[u8],
) -> bool {
    let mut text = std::str::from_utf8(line).unwrap_or("");
    while text.ends_with('\r') {
        text = &text[..text.len() - 1];
    }
    match needle {
        Some(needle) => {
            if text.to_ascii_lowercase().contains(needle.as_str()) {
                // Enforce the output limit even when the first matching line
                // is oversized: truncate to `max` bytes and stop reading.
                if text.len() > max {
                    let mut end = max.min(text.len());
                    while end > 0 && !text.is_char_boundary(end) {
                        end -= 1;
                    }
                    kept.push(text[..end].to_string());
                    *kept_bytes = end;
                    return false;
                }
                if *kept_bytes + text.len() + 1 > max && *kept_bytes > 0 {
                    return false;
                }
                *kept_bytes += text.len() + 1;
                kept.push(text.to_string());
            }
        }
        None => {
            if text.len() > max {
                let mut end = max.min(text.len());
                while end > 0 && !text.is_char_boundary(end) {
                    end -= 1;
                }
                kept.push(text[..end].to_string());
                *kept_bytes = end;
                return false;
            }
            if *kept_bytes + text.len() + 1 > max && *kept_bytes > 0 {
                return false;
            }
            *kept_bytes += text.len() + 1;
            kept.push(text.to_string());
        }
    }
    true
}

/// Open `file` for reading without following symlinks on *any* path
/// component: the digest root is opened once as a directory handle, then each
/// component of the relative path is traversed with openat+O_NOFOLLOW, so a
/// symlink swapped into any intermediate directory between the digest-path
/// check and the open cannot redirect the read outside the permitted root.
/// Every component is validated against an open handle rather than re-resolved
/// by pathname.
#[cfg(unix)]
fn open_digest_file(root: &Path, file: &Path) -> Result<std::fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let root_fd = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)
        .map_err(|e| format!("cannot open digest root {}: {e}", root.display()))?;
    let comps: Vec<&std::ffi::OsStr> = file
        .strip_prefix(root)
        .map_err(|e| format!("cannot compute path relative to {}: {e}", root.display()))?
        .components()
        .map(|c| c.as_os_str())
        .collect();
    if comps.is_empty() {
        return Err(format!(
            "cannot read {}: empty path relative to digest root",
            file.display()
        ));
    }
    let (dirs, last) = comps.split_at(comps.len() - 1);
    let mut dir_fd = root_fd;
    for comp in dirs {
        let cname =
            CString::new(comp.as_encoded_bytes()).map_err(|e| {
                format!("invalid path component {}: {e}", comp.to_string_lossy())
            })?;
        let next = unsafe {
            libc::openat(
                dir_fd.as_raw_fd(),
                cname.as_ptr(),
                libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if next < 0 {
            let e = std::io::Error::last_os_error();
            return Err(format!(
                "cannot traverse component {}: {e}",
                comp.to_string_lossy()
            ));
        }
        dir_fd = unsafe { std::fs::File::from_raw_fd(next) };
    }
    let cname = CString::new(last[0].as_encoded_bytes()).map_err(|e| {
        format!("invalid path component {}: {e}", last[0].to_string_lossy())
    })?;
    let fd = unsafe {
        libc::openat(
            dir_fd.as_raw_fd(),
            cname.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        let e = std::io::Error::last_os_error();
        return Err(format!("cannot read {}: {e}", file.display()));
    }
    Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}

fn read_excerpt(
    backend: &Backend,
    path: &str,
    contains: Option<&str>,
    tail: bool,
) -> Result<Excerpt, String> {
    let file = digest_path(&backend.config.digest_root, path)?;
    let max = backend.config.max_bytes;
    let mut note = format!("[{path}");

    // Open without following symlinks on the final component (Unix): a link
    // planted between the digest-path check and the open cannot redirect the
    // read outside the permitted root.
    // * Windows has no `openat`, so the guarantee comes from the handle instead
    // * of the name: the file is opened, then the handle is asked where it
    // * landed and refused if that is outside the digest root. A reparse point
    // * planted after `digest_path` validated the string cannot redirect the
    // * read, because the check describes the object already open.
    #[cfg(unix)]
    let f = open_digest_file(&backend.config.digest_root, &file)?;
    #[cfg(windows)]
    let f = {
        let root = &backend.config.digest_root;
        let relative = file
            .strip_prefix(root)
            .map_err(|_| format!("{} is not under the digest root", file.display()))?;
        silica_vault::win_fs::open_inside(root, relative)
            .map_err(|e| format!("cannot read {path}: {e}"))?
    };

    let meta = f.metadata().map_err(|e| format!("cannot stat {path}: {e}"))?;
    // Reject non-regular files (FIFOs, sockets, devices): they cannot be
    // meaningfully sought and may block or stream indefinitely.
    if !meta.file_type().is_file() {
        return Err(format!("{path} is not a regular file; refuse to read"));
    }
    let len = meta.len();

    if tail {
        // Seek to the last `max` bytes of a regular file: bounded disk
        // activity and memory even for a very large file or one enormous line
        // without newlines.
        let mut reader = std::io::BufReader::new(f);
        let start = len.saturating_sub(max as u64);
        reader
            .seek(std::io::SeekFrom::Start(start))
            .map_err(|e| format!("cannot seek {path}: {e}"))?;
        let mut tail_buf = Vec::new();
        reader
            .take(max as u64)
            .read_to_end(&mut tail_buf)
            .map_err(|e| format!("cannot read {path}: {e}"))?;
        let detail = format!(
            ": {len} bytes in file, last {} bytes kept",
            tail_buf.len()
        );
        note.push_str(&detail);
        note.push(']');
        return Ok(Excerpt {
            body: String::from_utf8_lossy(&tail_buf).into_owned(),
            note,
        });
    }

    // Head or contains mode: read at most `max` bytes from disk, limiting
    // each read to the remaining allowance so a single read cannot allocate
    // beyond the cap.
    let mut reader = std::io::BufReader::new(f);
    let mut bytes_read = 0u64;
    const CHUNK: usize = 8 << 10;
    let mut buf = [0u8; CHUNK];
    let needle = contains.filter(|n| !n.is_empty()).map(|n| n.to_lowercase());
    let mut kept: Vec<String> = Vec::new();
    let mut kept_bytes = 0usize;
    let mut partial: Vec<u8> = Vec::with_capacity(CHUNK);
    let mut stopped_early = false;

    loop {
        if bytes_read >= max as u64 {
            break;
        }
        let remaining = (max as u64 - bytes_read) as usize;
        let n = match reader.read(&mut buf[..remaining.min(CHUNK)]) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => return Err(format!("cannot read {path}: {e}")),
        };
        bytes_read += n as u64;
        let data = &buf[..n];
        let mut pos = 0;
        while pos < data.len() {
            match data[pos..].iter().position(|b| *b == b'\n') {
                Some(at) => {
                    let end = pos + at;
                    let mut line = partial.clone();
                    line.extend_from_slice(&data[pos..end]);
                    if !keep_line(&needle, &mut kept, &mut kept_bytes, max, &line) {
                        stopped_early = true;
                        break;
                    }
                    partial.clear();
                    pos = end + 1;
                }
                None => {
                    partial.extend_from_slice(&data[pos..]);
                    pos = data.len();
                }
            }
        }
        if stopped_early {
            break;
        }
    }

    // Any bytes left in `partial` form the final unterminated line. Emit it
    // only when nothing else was kept (an oversized first line with no
    // newline); otherwise it is an incomplete line read past the cutoff.
    if !stopped_early && !partial.is_empty() && kept.is_empty() {
        let mut line = partial;
        if line.len() > max {
            line.truncate(max);
        }
        keep_line(&needle, &mut kept, &mut kept_bytes, max, &line);
    }

    let (body, detail) = match &needle {
        Some(needle) => {
            let count = kept.len();
            if count == 0 {
                return Err(format!(
                    "{path} has no lines containing {needle:?}; nothing to summarize"
                ));
            }
            let body = kept.join("\n");
            let detail = format!(
                ": {bytes_read} bytes read, {count} lines match {needle:?}"
            );
            (body, detail)
        }
        None => {
            let detail = format!(
                ": {bytes_read} bytes read, {} lines kept",
                kept.len()
            );
            (kept.join("\n"), detail)
        }
    };
    note.push_str(&detail);
    note.push(']');
    Ok(Excerpt { body, note })
}


fn required_str<'a>(arguments: &'a Value, name: &str) -> Result<&'a str, String> {
    str_arg(arguments, name).ok_or_else(|| format!("`{name}` is required and must be a string"))
}

fn str_arg<'a>(arguments: &'a Value, name: &str) -> Option<&'a str> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn text(body: impl Into<String>) -> Value {
    json!({"type": "text", "text": body.into()})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Config;

    fn backend_with(max_bytes: usize) -> Backend {
        let mut config = Config::from_env().expect("the default config parses");
        config.max_bytes = max_bytes;
        // * Tests read files under the temp dir, so bound the digest root to it.
        config.digest_root = std::env::temp_dir();
        Backend::new(config)
    }

    #[test]
    fn every_tool_has_a_schema_and_a_description() {
        let listed = list();
        let tools = listed["tools"].as_array().unwrap();
        assert_eq!(tools.len(), TOOLS.len());
        for tool in tools {
            assert!(!tool["description"].as_str().unwrap().is_empty());
            assert_eq!(tool["inputSchema"]["type"], "object");
        }
    }

    #[test]
    fn an_unknown_tool_and_bad_arguments_are_faults() {
        let backend = backend_with(1 << 10);
        assert_eq!(
            call(&backend, &json!({"name": "llm_dance"}))
                .unwrap_err()
                .code,
            INVALID_PARAMS
        );
        assert_eq!(call(&backend, &json!({})).unwrap_err().code, INVALID_PARAMS);
        assert_eq!(
            call(&backend, &json!({"name": "llm_generate", "arguments": 7}))
                .unwrap_err()
                .code,
            INVALID_PARAMS
        );
    }

    #[test]
    fn a_missing_prompt_is_a_readable_tool_error_not_a_fault() {
        let backend = backend_with(1 << 10);
        let result = call(&backend, &json!({"name": "llm_generate", "arguments": {}})).unwrap();
        assert_eq!(result["isError"], json!(true));
        assert!(result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("`prompt` is required"));
    }

    #[test]
    fn digest_filters_and_truncates_and_says_so() {
        let dir = std::env::temp_dir().join("silica-llm-mcp-digest");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("log.txt");
        std::fs::write(&path, "ok one\nFAILED two\nok three\nfailed four\n").unwrap();
        let path = path.to_string_lossy().into_owned();

        let whole = read_excerpt(&backend_with(1 << 20), &path, None, false).unwrap();
        assert!(whole.note.contains("lines kept"), "{}", whole.note);
        assert!(whole.body.contains("ok one"));

        // * The filter is case-insensitive, and the note records the count.
        let only = read_excerpt(&backend_with(1 << 20), &path, Some("failed"), false).unwrap();
        assert_eq!(only.body, "FAILED two\nfailed four");
        assert!(only.note.contains("lines match"), "{}", only.note);

        // * Truncation says which end was kept.
        let head = read_excerpt(&backend_with(10), &path, None, false).unwrap();
        assert!(head.note.contains("bytes read"), "{}", head.note);
        assert!(head.body.len() <= 10);
        let tail = read_excerpt(&backend_with(10), &path, None, true).unwrap();
        assert!(tail.note.contains("last 10 bytes"), "{}", tail.note);
        assert!(tail.body.ends_with("four\n"));

        // * A filter that matches nothing is an error, not an empty summary.
        let none = read_excerpt(&backend_with(1 << 20), &path, Some("zzz"), false);
        assert!(none.unwrap_err().contains("no lines containing"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn long_single_line_is_bounded_by_max_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-longline-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("long.txt");
        // One line with no newline, far larger than the limit.
        std::fs::write(&path, "x".repeat(100_000)).unwrap();
        let path = path.to_string_lossy().into_owned();

        // Head mode: limit smaller than the line -> body bounded to max bytes.
        let head = read_excerpt(&backend_with(50), &path, None, false).unwrap();
        assert!(head.body.len() <= 50, "{}", head.body.len());
        assert!(head.body.starts_with("x"));

        // Tail mode: single enormous line -> memory bounded.
        let tail = read_excerpt(&backend_with(50), &path, None, true).unwrap();
        assert!(tail.body.len() <= 50, "{}", tail.body.len());
        assert!(tail.body.ends_with("x"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn contains_mode_stops_at_byte_limit_without_allocating_a_giant_line() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-containlong-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("log.txt");
        // Every line is long and none contains the needle; the limit stops us
        // before we ever reach a later matching line, and no giant line is
        // allocated.
        std::fs::write(&path, "hello world\n".repeat(20_000)).unwrap();
        let path = path.to_string_lossy().into_owned();

        let err = read_excerpt(&backend_with(50), &path, Some("match"), false)
            .unwrap_err();
        assert!(err.contains("no lines containing"), "{err}");
    }

    #[test]
    fn file_without_newline_and_match_within_limit() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-nonewline-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("note.txt");
        std::fs::write(&path, "abcdefghij").unwrap();
        let path = path.to_string_lossy().into_owned();

        let out = read_excerpt(&backend_with(100), &path, Some("abc"), false).unwrap();
        assert_eq!(out.body, "abcdefghij");
        assert!(out.note.contains("lines match"), "{}", out.note);
    }

    #[test]
    fn digest_root_rejects_files_outside_it() {
        use std::env::temp_dir;
        use std::fs;
        let base = temp_dir().join(format!(
            "silica-digest-root-{}",
            std::process::id()
        ));
        fs::create_dir_all(base.join("root/inside")).unwrap();
        fs::create_dir_all(base.join("outside")).unwrap();
        fs::write(base.join("root/inside/note.md"), "inside note").unwrap();
        fs::write(base.join("outside/secret.md"), "secret").unwrap();
        let mut config = Config::from_env().expect("config parses");
        config.max_bytes = 1 << 20;
        config.digest_root = base.join("root");
        let backend = Backend::new(config);
        let inside = base.join("root/inside/note.md");
        let out =
            read_excerpt(&backend, inside.to_str().unwrap(), None, false).unwrap();
        assert!(out.body.contains("inside note"), "{}", out.body);
        let outside = base.join("outside/secret.md");
        let err = read_excerpt(&backend, outside.to_str().unwrap(), None, false)
            .unwrap_err();
        assert!(
            err.contains("outside the permitted digest root"),
            "{err}"
        );
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_missing_file_is_reported_with_its_path() {
        let e =
            read_excerpt(&backend_with(1 << 10), "/nonexistent/log.txt", None, false).unwrap_err();
        assert!(e.contains("/nonexistent/log.txt"), "{e}");
    }

    #[test]
    fn digest_head_caps_output_at_max_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-headcap-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.txt");
        std::fs::write(&path, "alpha\nbeta\ngamma\ndelta\n").unwrap();
        let path = path.to_string_lossy().into_owned();

        let head = read_excerpt(&backend_with(20), &path, None, false).unwrap();
        assert!(head.body.len() <= 20, "{}", head.body.len());
        assert!(head.body.starts_with("alpha\nbeta"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn digest_tail_reads_only_last_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-tail-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.txt");
        std::fs::write(&path, "0123456789\nabcdefghij\n").unwrap();
        let path = path.to_string_lossy().into_owned();

        let tail = read_excerpt(&backend_with(7), &path, None, true).unwrap();
        assert!(tail.body.len() <= 7, "{}", tail.body.len());
        assert!(tail.body.ends_with("ghij\n"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn digest_rejects_non_regular_file() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-dir-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let backend = backend_with(1 << 20);
        let err = read_excerpt(&backend, dir.to_str().unwrap(), None, false)
            .unwrap_err();
        assert!(
            err.contains("is not a regular file"),
            "{err}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }


    #[test]
    fn digest_oversized_first_line_capped_at_limit() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-oversize-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("one_line.txt");
        std::fs::write(&path, "z".repeat(10_000)).unwrap();
        let path = path.to_string_lossy().into_owned();

        let head = read_excerpt(&backend_with(20), &path, None, false).unwrap();
        assert!(head.body.len() <= 20, "{}", head.body.len());
        assert!(head.body.starts_with("z"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn digest_exact_limit_file() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-exact-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("exact.txt");
        std::fs::write(&path, "0123456789").unwrap();
        let path = path.to_string_lossy().into_owned();

        let head = read_excerpt(&backend_with(10), &path, None, false).unwrap();
        assert_eq!(head.body, "0123456789");
        std::fs::remove_dir_all(&dir).ok();
    }


    #[test]
    #[cfg(unix)]
    fn digest_openat_traverses_nested_file() {
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-openat-{0}",
            std::process::id()
        ));
        std::fs::create_dir_all(dir.join("sub/dir")).unwrap();
        std::fs::write(dir.join("sub/dir/note.txt"), "nested content").unwrap();
        let file = dir.join("sub/dir/note.txt");
        let f = open_digest_file(&dir, &file).unwrap();
        assert!(f.metadata().unwrap().file_type().is_file());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn digest_openat_rejects_symlinked_intermediate_directory() {
        use std::os::unix::fs::symlink;
        let dir = std::env::temp_dir().join(format!(
            "silica-llm-mcp-openat-sym-{0}",
            std::process::id()
        ));
        let real = std::env::temp_dir().join(format!(
            "silica-llm-mcp-openat-real-{0}",
            std::process::id()
        ));
        std::fs::create_dir_all(real.join("note")).unwrap();
        std::fs::write(real.join("note/secret.txt"), "secret").unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        symlink(&real, dir.join("link")).unwrap();
        std::fs::write(dir.join("own.txt"), "own content").unwrap();
        let err = open_digest_file(&dir, &dir.join("link/note/secret.txt"))
            .unwrap_err();
        assert!(
            err.contains("cannot traverse component"),
            "{err}"
        );
        let f = open_digest_file(&dir, &dir.join("own.txt")).unwrap();
        assert!(f.metadata().unwrap().file_type().is_file());
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&real).ok();
    }
}
