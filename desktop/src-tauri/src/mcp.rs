//! Bounded, read-only MCP stdio transport for the desktop binary.

use serde_json::{Map, Value, json};
use std::{
    collections::HashSet,
    io::{self, BufRead, BufReader, Write},
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread,
    time::{Duration, Instant},
};

const MODERN_PROTOCOL_VERSION: &str = "2026-07-28";
const LEGACY_PROTOCOL_VERSION: &str = "2025-11-25";
const MAX_FRAME_BYTES: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_TEXT_CONTENT_BYTES: usize = 128 * 1024;
const MAX_PENDING_JOBS: usize = 16;
const EOF_DRAIN_TIMEOUT: Duration = Duration::from_millis(250);

/// Host-side dispatcher for the fixed, read-only OpenDrobo MCP tool set.
pub trait Backend: Send + 'static {
    fn call(&mut self, name: &str, args: &Map<String, Value>) -> Result<Value, String>;
}

#[derive(Default)]
struct Session {
    legacy_initialize_accepted: bool,
    legacy_initialized: bool,
}

/// Serve one newline-delimited JSON-RPC message at a time over standard I/O.
///
/// This server intentionally exposes no network transport, filesystem path,
/// shell, write, administrative, or dynamically-selected device operation.
pub fn serve_stdio<B: Backend>(backend: B) -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    serve(BufReader::new(stdin), &mut stdout.lock(), backend)
}

fn serve<R: BufRead + Send + 'static, W: Write, B: Backend>(
    reader: R,
    writer: &mut W,
    backend: B,
) -> Result<(), String> {
    let (input_sender, input_receiver) = mpsc::sync_channel(MAX_PENDING_JOBS);
    let reader_handle = thread::spawn(move || read_input(reader, input_sender));
    let (job_sender, job_receiver) = mpsc::sync_channel(MAX_PENDING_JOBS);
    let (completion_sender, completion_receiver) = mpsc::sync_channel(MAX_PENDING_JOBS);
    thread::spawn(move || run_worker(backend, job_receiver, completion_sender));

    let mut session = Session::default();
    let mut cancelled = HashSet::new();
    let mut pending_keys = HashSet::new();
    let mut pending = 0usize;
    loop {
        drain_completions(
            &completion_receiver,
            writer,
            &mut cancelled,
            &mut pending_keys,
            &mut pending,
        )?;
        match input_receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(Input::Frame(frame)) => match dispatch_frame(frame, &mut session) {
                Action::None => {}
                Action::Response(response) => write_response(writer, response)?,
                Action::Cancel(key) => {
                    if pending_keys.contains(&key) {
                        cancelled.insert(key);
                    }
                }
                Action::Job(job) => {
                    let key = job.key.clone();
                    match job_sender.try_send(job) {
                        Ok(()) => {
                            pending += 1;
                            pending_keys.insert(key);
                        }
                        Err(TrySendError::Full(job)) => {
                            write_response(writer, error_response(job.id, -32000, "Server busy"))?
                        }
                        Err(TrySendError::Disconnected(job)) => write_response(
                            writer,
                            error_response(job.id, -32603, "Server unavailable"),
                        )?,
                    }
                }
            },
            Ok(Input::Eof) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                drop(job_sender);
                drain_before_eof(
                    &completion_receiver,
                    writer,
                    &mut cancelled,
                    &mut pending_keys,
                    &mut pending,
                )?;
                let _ = reader_handle.join();
                return Ok(());
            }
            Ok(Input::Error(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

enum Input {
    Frame(Frame),
    Eof,
    Error(String),
}

struct Job {
    id: Value,
    key: String,
    name: String,
    args: Map<String, Value>,
    modern: bool,
}

struct Completion {
    key: String,
    response: Value,
}

enum Action {
    None,
    Response(Value),
    Cancel(String),
    Job(Job),
}

fn read_input<R: BufRead>(mut reader: R, sender: SyncSender<Input>) {
    loop {
        match read_frame(&mut reader) {
            Ok(Some(frame)) => {
                if sender.send(Input::Frame(frame)).is_err() {
                    return;
                }
            }
            Ok(None) => {
                let _ = sender.send(Input::Eof);
                return;
            }
            Err(error) => {
                let _ = sender.send(Input::Error(error));
                return;
            }
        }
    }
}

fn run_worker<B: Backend>(mut backend: B, jobs: Receiver<Job>, sender: SyncSender<Completion>) {
    for job in jobs {
        let result = match backend.call(&job.name, &job.args) {
            Ok(value) => tool_call_result(job.modern, value, false),
            Err(_) => tool_call_result(job.modern, Value::Null, true),
        };
        if sender
            .send(Completion {
                key: job.key,
                response: success_response(job.id, result),
            })
            .is_err()
        {
            return;
        }
    }
}

fn drain_completions<W: Write>(
    receiver: &Receiver<Completion>,
    writer: &mut W,
    cancelled: &mut HashSet<String>,
    pending_keys: &mut HashSet<String>,
    pending: &mut usize,
) -> Result<(), String> {
    while let Ok(completion) = receiver.try_recv() {
        *pending = pending.saturating_sub(1);
        pending_keys.remove(&completion.key);
        if !cancelled.remove(&completion.key) {
            write_response(writer, completion.response)?;
        }
    }
    Ok(())
}

fn drain_before_eof<W: Write>(
    receiver: &Receiver<Completion>,
    writer: &mut W,
    cancelled: &mut HashSet<String>,
    pending_keys: &mut HashSet<String>,
    pending: &mut usize,
) -> Result<(), String> {
    let deadline = Instant::now() + EOF_DRAIN_TIMEOUT;
    while *pending > 0 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        match receiver.recv_timeout(remaining) {
            Ok(completion) => {
                *pending = pending.saturating_sub(1);
                pending_keys.remove(&completion.key);
                if !cancelled.remove(&completion.key) {
                    write_response(writer, completion.response)?;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
                return Ok(());
            }
        }
    }
    Ok(())
}

enum Frame {
    Bytes(Vec<u8>),
    Oversized,
}

fn read_frame<R: BufRead>(reader: &mut R) -> Result<Option<Frame>, String> {
    let mut frame = Vec::new();
    let mut oversized = false;

    loop {
        let buffer = reader.fill_buf().map_err(|error| error.to_string())?;
        if buffer.is_empty() {
            if frame.is_empty() && !oversized {
                return Ok(None);
            }
            return Ok(Some(if oversized {
                Frame::Oversized
            } else {
                Frame::Bytes(frame)
            }));
        }

        if let Some(newline) = buffer.iter().position(|byte| *byte == b'\n') {
            if !oversized && frame.len().saturating_add(newline) <= MAX_FRAME_BYTES {
                frame.extend_from_slice(&buffer[..newline]);
            } else {
                oversized = true;
            }
            reader.consume(newline + 1);
            return Ok(Some(if oversized {
                Frame::Oversized
            } else {
                Frame::Bytes(frame)
            }));
        }

        if frame.len().saturating_add(buffer.len()) <= MAX_FRAME_BYTES && !oversized {
            frame.extend_from_slice(buffer);
        } else {
            oversized = true;
        }
        let consumed = buffer.len();
        reader.consume(consumed);
    }
}

fn dispatch_frame(frame: Frame, session: &mut Session) -> Action {
    match frame {
        Frame::Oversized => {
            Action::Response(error_response(Value::Null, -32600, "Request too large"))
        }
        Frame::Bytes(bytes) => match serde_json::from_slice::<Value>(&bytes) {
            Ok(message) => dispatch_message(message, session),
            Err(_) => Action::Response(error_response(Value::Null, -32700, "Parse error")),
        },
    }
}

fn dispatch_message(message: Value, session: &mut Session) -> Action {
    let Some(object) = message.as_object() else {
        return Action::Response(error_response(Value::Null, -32600, "Invalid request"));
    };
    if object.get("jsonrpc") != Some(&Value::String("2.0".to_string())) {
        return Action::Response(error_response(Value::Null, -32600, "Invalid request"));
    }
    let Some(method) = object.get("method").and_then(Value::as_str) else {
        return Action::Response(error_response(Value::Null, -32600, "Invalid request"));
    };

    let id = match object.get("id") {
        None => return handle_notification(method, object.get("params"), session),
        Some(id) if valid_id(id) => id.clone(),
        Some(_) => {
            return Action::Response(error_response(Value::Null, -32600, "Invalid request id"));
        }
    };
    let params = object.get("params");

    if method == "initialize" && !is_modern_request(params) {
        return Action::Response(handle_legacy_initialize(id, params, session));
    }

    let modern = is_modern_request(params);
    if modern {
        if let Err(response) = validate_modern_metadata(&id, params) {
            return Action::Response(response);
        }
        return handle_modern_request(id, method, params);
    }

    if has_meta(params) && !session.legacy_initialize_accepted {
        return Action::Response(error_response(id, -32600, "Invalid request metadata"));
    }
    if method == "initialize" {
        return Action::Response(error_response(
            id,
            -32600,
            "Modern requests do not use initialize",
        ));
    }
    if !session.legacy_initialized {
        return Action::Response(error_response(id, -32002, "Server not initialized"));
    }
    handle_legacy_request(id, method, params)
}

fn handle_notification(method: &str, params: Option<&Value>, session: &mut Session) -> Action {
    if method == "notifications/initialized"
        && session.legacy_initialize_accepted
        && params.map_or(true, Value::is_object)
    {
        session.legacy_initialized = true;
        return Action::None;
    }
    if method == "notifications/cancelled" {
        if let Some(key) = params
            .and_then(Value::as_object)
            .and_then(|params| params.get("requestId"))
            .filter(|id| valid_id(id))
            .map(request_key)
        {
            return Action::Cancel(key);
        }
    }
    Action::None
}

fn valid_id(id: &Value) -> bool {
    id.is_string() || id.is_number()
}

fn request_key(id: &Value) -> String {
    serde_json::to_string(id).expect("valid JSON-RPC IDs serialize")
}

fn has_meta(params: Option<&Value>) -> bool {
    params
        .and_then(Value::as_object)
        .is_some_and(|params| params.contains_key("_meta"))
}

fn is_modern_request(params: Option<&Value>) -> bool {
    params
        .and_then(Value::as_object)
        .and_then(|params| params.get("_meta"))
        .and_then(Value::as_object)
        .is_some_and(|meta| meta.contains_key("io.modelcontextprotocol/protocolVersion"))
}

fn validate_modern_metadata(id: &Value, params: Option<&Value>) -> Result<(), Value> {
    let Some(params) = params.and_then(Value::as_object) else {
        return Err(error_response(
            id.clone(),
            -32600,
            "Invalid request parameters",
        ));
    };
    let Some(meta) = params.get("_meta").and_then(Value::as_object) else {
        return Err(error_response(
            id.clone(),
            -32600,
            "Invalid request metadata",
        ));
    };
    let Some(version) = meta
        .get("io.modelcontextprotocol/protocolVersion")
        .and_then(Value::as_str)
    else {
        return Err(error_response(
            id.clone(),
            -32600,
            "Invalid request metadata",
        ));
    };
    if !meta
        .get("io.modelcontextprotocol/clientCapabilities")
        .is_some_and(Value::is_object)
    {
        return Err(error_response(
            id.clone(),
            -32600,
            "Invalid request metadata",
        ));
    }
    if version != MODERN_PROTOCOL_VERSION {
        return Err(error_response_with_data(
            id.clone(),
            -32022,
            "Unsupported protocol version",
            json!({
                "supported": [MODERN_PROTOCOL_VERSION],
                "requested": version,
            }),
        ));
    }
    Ok(())
}

fn handle_modern_request(id: Value, method: &str, params: Option<&Value>) -> Action {
    match method {
        "server/discover" => {
            if !only_keys(params, &["_meta"]) {
                return Action::Response(error_response(id, -32602, "Invalid parameters"));
            }
            Action::Response(success_response(
                id,
                json!({
                    "resultType": "complete",
                    "supportedVersions": [MODERN_PROTOCOL_VERSION, LEGACY_PROTOCOL_VERSION],
                    "capabilities": { "tools": { "listChanged": false } },
                    "instructions": "Read-only OpenDrobo inspection tools only. No client-selected file access, shell, network listener, device-control execution, or administrative tools. Inspection may write bounded local app logs and capacity history. Service-derived readings use unauthenticated local IPC; they are not security-attested device state.",
                    "ttlMs": 0,
                    "cacheScope": "private",
                }),
            ))
        }
        "tools/list" => {
            if !only_keys(params, &["_meta"]) {
                return Action::Response(error_response(id, -32602, "Invalid parameters"));
            }
            Action::Response(success_response(id, tools_list_result(true)))
        }
        "tools/call" => handle_tool_call(id, params, true),
        _ => Action::Response(error_response(id, -32601, "Method not found")),
    }
}

fn handle_legacy_initialize(id: Value, params: Option<&Value>, session: &mut Session) -> Value {
    if session.legacy_initialize_accepted || !legacy_initialize_params_valid(params) {
        return error_response(id, -32602, "Invalid initialize parameters");
    }
    session.legacy_initialize_accepted = true;
    success_response(
        id,
        json!({
            "protocolVersion": LEGACY_PROTOCOL_VERSION,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "opendrobo", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Read-only OpenDrobo inspection tools only. Inspection may write bounded local app logs and capacity history. Service-derived readings use unauthenticated local IPC; they are not security-attested device state.",
        }),
    )
}

fn legacy_initialize_params_valid(params: Option<&Value>) -> bool {
    let Some(params) = params.and_then(Value::as_object) else {
        return false;
    };
    if !params.keys().all(|key| {
        matches!(
            key.as_str(),
            "protocolVersion" | "capabilities" | "clientInfo" | "_meta"
        )
    }) {
        return false;
    }
    params.get("protocolVersion") == Some(&Value::String(LEGACY_PROTOCOL_VERSION.to_string()))
        && params.get("capabilities").is_some_and(Value::is_object)
        && params
            .get("clientInfo")
            .and_then(Value::as_object)
            .is_some_and(|info| {
                info.get("name").is_some_and(Value::is_string)
                    && info.get("version").is_some_and(Value::is_string)
            })
}

fn handle_legacy_request(id: Value, method: &str, params: Option<&Value>) -> Action {
    match method {
        "tools/list" => {
            if !only_keys(params, &["_meta"]) {
                return Action::Response(error_response(id, -32602, "Invalid parameters"));
            }
            Action::Response(success_response(id, tools_list_result(false)))
        }
        "tools/call" => handle_tool_call(id, params, false),
        _ => Action::Response(error_response(id, -32601, "Method not found")),
    }
}

fn only_keys(params: Option<&Value>, allowed: &[&str]) -> bool {
    params.map_or(true, |params| {
        params.as_object().is_some_and(|params| {
            params
                .keys()
                .all(|key| allowed.iter().any(|allowed| key == allowed))
        })
    })
}

fn handle_tool_call(id: Value, params: Option<&Value>, modern: bool) -> Action {
    let allowed = &["name", "arguments", "_meta"][..];
    if !only_keys(params, allowed) {
        return Action::Response(error_response(id, -32602, "Invalid parameters"));
    }
    let Some(params) = params.and_then(Value::as_object) else {
        return Action::Response(error_response(id, -32602, "Invalid parameters"));
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Action::Response(error_response(id, -32602, "Invalid tool name"));
    };
    let args = match params.get("arguments") {
        None => Map::new(),
        Some(Value::Object(args)) => args.clone(),
        Some(_) => return Action::Response(error_response(id, -32602, "Invalid tool arguments")),
    };
    if !valid_tool_arguments(name, &args) {
        return Action::Response(error_response(id, -32602, "Invalid tool arguments"));
    }
    Action::Job(Job {
        key: request_key(&id),
        id,
        name: name.to_string(),
        args,
        modern,
    })
}

fn tool_call_result(modern: bool, value: Value, is_error: bool) -> Value {
    let text = if is_error {
        "Read-only query failed."
    } else {
        "Read-only query completed."
    };
    let compatible_text = if is_error {
        text.to_string()
    } else {
        bounded_json_text(&value)
    };
    let mut result = json!({
        "content": [{ "type": "text", "text": compatible_text }],
        "isError": is_error,
    });
    if !is_error && (modern || value.is_object()) {
        result["structuredContent"] = value;
    }
    if modern {
        result["resultType"] = Value::String("complete".to_string());
    }
    result
}

fn bounded_json_text(value: &Value) -> String {
    match serde_json::to_string(value) {
        Ok(text) if text.len() <= MAX_TEXT_CONTENT_BYTES => text,
        _ => "{\"truncated\":true,\"message\":\"Result exceeds text compatibility limit.\"}"
            .to_string(),
    }
}

fn valid_tool_arguments(name: &str, args: &Map<String, Value>) -> bool {
    match name {
        "opendrobo_inventory"
        | "opendrobo_pool_status"
        | "opendrobo_storage_status"
        | "opendrobo_sharing_status"
        | "opendrobo_controls"
        | "opendrobo_capabilities" => args.is_empty(),
        "opendrobo_read_capacity" => {
            if args.len() != 1 {
                return false;
            }
            let Some(device_id) = args.get("deviceId").and_then(Value::as_str) else {
                return false;
            };
            strict_nonzero_u64(device_id)
        }
        "opendrobo_console" | "opendrobo_events" => {
            if args.len() > 1 || !args.keys().all(|key| key == "limit") {
                return false;
            }
            args.get("limit")
                .map(|limit| {
                    limit
                        .as_u64()
                        .is_some_and(|limit| (1..=200).contains(&limit))
                })
                .unwrap_or(true)
        }
        "opendrobo_capacity_history" => {
            if args.keys().any(|key| key != "limit" && key != "unitId") {
                return false;
            }
            let limit = args
                .get("limit")
                .is_none_or(|value| value.as_u64().is_some_and(|n| (1..=2048).contains(&n)));
            let id = args.get("unitId").is_none_or(|value| {
                value.as_str().is_some_and(|id| {
                    id.strip_prefix("opendrobo/device/").is_some_and(|hash| {
                        hash.len() == 64
                            && hash
                                .bytes()
                                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                    })
                })
            });
            limit && id
        }
        _ => false,
    }
}

fn strict_nonzero_u64(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 20
        && value.as_bytes()[0] != b'0'
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok_and(|value| value != 0)
}

fn tools_list_result(modern: bool) -> Value {
    let mut result = json!({
        "tools": tools(),
    });
    if modern {
        result["resultType"] = Value::String("complete".to_string());
        result["ttlMs"] = Value::Number(0.into());
        result["cacheScope"] = Value::String("private".to_string());
    }
    result
}

fn tools() -> Vec<Value> {
    vec![
        tool(
            "opendrobo_inventory",
            "List discovered OpenDrobo devices.",
            empty_schema(),
        ),
        tool(
            "opendrobo_read_capacity",
            "Read capacity for one discovered device ID.",
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "deviceId": {
                        "type": "string",
                        "pattern": "^[1-9][0-9]{0,19}$",
                        "description": "Nonzero decimal u64 device ID."
                    }
                },
                "required": ["deviceId"]
            }),
        ),
        tool(
            "opendrobo_pool_status",
            "Read cached OpenDrobo pool status.",
            empty_schema(),
        ),
        tool(
            "opendrobo_storage_status",
            "Read storage status.",
            empty_schema(),
        ),
        tool(
            "opendrobo_sharing_status",
            "Read sharing status.",
            empty_schema(),
        ),
        tool(
            "opendrobo_capabilities",
            "Read OpenDrobo capabilities.",
            empty_schema(),
        ),
        tool(
            "opendrobo_console",
            "Read bounded sanitized console entries.",
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 }
                }
            }),
        ),
        tool(
            "opendrobo_events",
            "Read bounded device events observed in this MCP process after a pool read. Not onboard history.",
            json!({"type":"object","additionalProperties":false,"properties":{
                "limit":{"type":"integer","minimum":1,"maximum":200}
            }}),
        ),
        tool(
            "opendrobo_controls",
            "Inspect GUI-only control availability. This tool cannot execute a command.",
            empty_schema(),
        ),
        tool(
            "opendrobo_capacity_history",
            "Read locally recorded protected-pool observations. No onboard history or missing past samples are fabricated.",
            json!({"type":"object","additionalProperties":false,"properties":{
                "unitId":{"type":"string","pattern":"^opendrobo/device/[0-9a-f]{64}$"},
                "limit":{"type":"integer","minimum":1,"maximum":2048}
            }}),
        ),
    ]
}

fn empty_schema() -> Value {
    json!({ "type": "object", "additionalProperties": false })
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false,
        }
    })
}

fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

fn error_response_with_data(id: Value, code: i64, message: &str, data: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message, "data": data },
    })
}

fn write_response<W: Write>(writer: &mut W, response: Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(&response).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_OUTPUT_BYTES {
        let id = response
            .get("id")
            .cloned()
            .filter(valid_id)
            .unwrap_or(Value::Null);
        let fallback = serde_json::to_vec(&error_response(id, -32603, "Response too large"))
            .map_err(|error| error.to_string())?;
        writer
            .write_all(&fallback)
            .map_err(|error| error.to_string())?;
    } else {
        writer
            .write_all(&bytes)
            .map_err(|error| error.to_string())?;
    }
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Cursor, Read},
        sync::{Arc, Mutex, mpsc},
    };

    #[derive(Clone, Default)]
    struct MockBackend {
        calls: Arc<Mutex<Vec<(String, Map<String, Value>)>>>,
    }

    impl Backend for MockBackend {
        fn call(&mut self, name: &str, args: &Map<String, Value>) -> Result<Value, String> {
            self.calls
                .lock()
                .expect("calls lock")
                .push((name.to_string(), args.clone()));
            Ok(json!({ "ok": true, "name": name }))
        }
    }

    struct SizedBackend {
        value: Value,
        calls: Arc<Mutex<usize>>,
    }

    impl Backend for SizedBackend {
        fn call(&mut self, _: &str, _: &Map<String, Value>) -> Result<Value, String> {
            *self.calls.lock().expect("calls lock") += 1;
            Ok(self.value.clone())
        }
    }

    struct ChannelReader {
        receiver: mpsc::Receiver<Vec<u8>>,
        buffer: Vec<u8>,
        offset: usize,
    }

    impl Read for ChannelReader {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            let available = self.fill_buf()?;
            let count = available.len().min(output.len());
            output[..count].copy_from_slice(&available[..count]);
            self.consume(count);
            Ok(count)
        }
    }

    impl BufRead for ChannelReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            while self.offset == self.buffer.len() {
                match self.receiver.recv() {
                    Ok(buffer) => {
                        self.buffer = buffer;
                        self.offset = 0;
                    }
                    Err(_) => return Ok(&[]),
                }
            }
            Ok(&self.buffer[self.offset..])
        }

        fn consume(&mut self, amount: usize) {
            self.offset = self.offset.saturating_add(amount).min(self.buffer.len());
        }
    }

    #[derive(Clone)]
    struct SharedWriter {
        bytes: Arc<Mutex<Vec<u8>>>,
        complete: mpsc::Sender<()>,
    }

    impl Write for SharedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.bytes
                .lock()
                .expect("writer lock")
                .extend_from_slice(bytes);
            if bytes == b"\n" {
                let _ = self.complete.send(());
            }
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct BlockingBackend {
        started: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
        finished: mpsc::Sender<()>,
    }

    impl Backend for BlockingBackend {
        fn call(&mut self, _: &str, _: &Map<String, Value>) -> Result<Value, String> {
            let _ = self.started.send(());
            let _ = self.release.recv();
            let _ = self.finished.send(());
            Ok(json!({ "completed": true }))
        }
    }

    fn modern(method: &str, id: u64, extra: Value) -> Value {
        let mut params = extra.as_object().cloned().expect("object params");
        params.insert(
            "_meta".to_string(),
            json!({
                "io.modelcontextprotocol/protocolVersion": MODERN_PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientCapabilities": {},
            }),
        );
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    }

    fn run(input: Vec<Value>, backend: MockBackend) -> Vec<Value> {
        let wire = input
            .into_iter()
            .map(|value| serde_json::to_string(&value).expect("request serializes"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut output = Vec::new();
        serve(
            Cursor::new(format!("{wire}\n").into_bytes()),
            &mut output,
            backend,
        )
        .expect("server exits at EOF");
        String::from_utf8(output)
            .expect("response is UTF-8")
            .lines()
            .map(|line| serde_json::from_str(line).expect("response is JSON"))
            .collect()
    }

    #[test]
    fn invalid_uninitialized_and_mutating_requests_never_reach_backend() {
        let backend = MockBackend::default();
        let responses = run(
            vec![
                modern(
                    "tools/call",
                    1,
                    json!({ "name": "opendrobo_read_capacity", "arguments": { "deviceId": "0" } }),
                ),
                json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "opendrobo_inventory" } }),
                modern(
                    "tools/call",
                    3,
                    json!({ "name": "filesystem/write", "arguments": {} }),
                ),
            ],
            backend.clone(),
        );

        assert!(backend.calls.lock().expect("calls lock").is_empty());
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0]["error"]["code"], -32602);
        assert_eq!(responses[1]["error"]["code"], -32002);
        assert_eq!(responses[2]["error"]["code"], -32602);
    }

    #[test]
    fn modern_discovery_list_and_call_have_correct_framed_responses() {
        let backend = MockBackend::default();
        let responses = run(
            vec![
                modern("server/discover", 1, json!({})),
                modern("tools/list", 2, json!({})),
                modern(
                    "tools/call",
                    3,
                    json!({
                        "name": "opendrobo_read_capacity",
                        "arguments": { "deviceId": "42" }
                    }),
                ),
            ],
            backend.clone(),
        );

        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0]["result"]["resultType"], "complete");
        assert_eq!(
            responses[0]["result"]["supportedVersions"][0],
            MODERN_PROTOCOL_VERSION
        );
        assert_eq!(
            responses[1]["result"]["tools"]
                .as_array()
                .expect("tools")
                .len(),
            10
        );
        let names: Vec<&str> = responses[1]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "opendrobo_inventory",
                "opendrobo_read_capacity",
                "opendrobo_pool_status",
                "opendrobo_storage_status",
                "opendrobo_sharing_status",
                "opendrobo_capabilities",
                "opendrobo_console",
                "opendrobo_events",
                "opendrobo_controls",
                "opendrobo_capacity_history"
            ]
        );
        assert_eq!(
            responses[1]["result"]["tools"][0]["annotations"]["readOnlyHint"],
            true
        );
        assert_eq!(responses[2]["result"]["resultType"], "complete");
        let text: Value = serde_json::from_str(
            responses[2]["result"]["content"][0]["text"]
                .as_str()
                .expect("JSON text content"),
        )
        .expect("compatible content is JSON");
        assert_eq!(text, responses[2]["result"]["structuredContent"]);
        let calls = backend.calls.lock().expect("calls lock");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "opendrobo_read_capacity");
        assert_eq!(calls[0].1["deviceId"], "42");
    }

    #[test]
    fn history_invalid_scope_never_reaches_backend() {
        let backend = MockBackend::default();
        let cases = [
            json!({"limit":0}),
            json!({"limit":2049}),
            json!({"unitId":format!("opendrobo/device/{}","A".repeat(64))}),
            json!({"unitId":"pool-1"}),
            json!({"path":"/private/capacity-history.jsonl"}),
        ];
        let requests = cases
            .into_iter()
            .enumerate()
            .map(|(index, args)| {
                modern(
                    "tools/call",
                    index as u64 + 1,
                    json!({"name":"opendrobo_capacity_history","arguments":args}),
                )
            })
            .collect();
        let responses = run(requests, backend.clone());
        assert_eq!(responses.len(), 5);
        assert!(
            responses
                .iter()
                .all(|response| response.get("error").is_some())
        );
        assert!(backend.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn legacy_initialize_then_initialized_enables_legacy_tools() {
        let backend = MockBackend::default();
        let responses = run(
            vec![
                json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": LEGACY_PROTOCOL_VERSION,
                        "capabilities": {},
                        "clientInfo": { "name": "test-client", "version": "1" }
                    }
                }),
                json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
                json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": { "_meta": {} } }),
                json!({
                    "jsonrpc": "2.0",
                    "id": 3,
                    "method": "tools/call",
                    "params": {
                        "name": "opendrobo_inventory",
                        "arguments": {},
                        "_meta": {}
                    }
                }),
            ],
            backend.clone(),
        );

        assert_eq!(responses.len(), 3);
        assert_eq!(
            responses[0]["result"]["protocolVersion"],
            LEGACY_PROTOCOL_VERSION
        );
        assert!(responses[1]["result"].get("resultType").is_none());
        assert!(responses[2]["result"].get("resultType").is_none());
        let text: Value = serde_json::from_str(
            responses[2]["result"]["content"][0]["text"]
                .as_str()
                .expect("JSON text content"),
        )
        .expect("legacy-compatible content is JSON");
        assert_eq!(text, responses[2]["result"]["structuredContent"]);
        let calls = backend.calls.lock().expect("calls lock");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "opendrobo_inventory");
    }

    #[test]
    fn pool_status_is_a_fixed_read_only_no_argument_tool() {
        let backend = MockBackend::default();
        let responses = run(
            vec![
                modern("tools/list", 1, json!({})),
                modern(
                    "tools/call",
                    2,
                    json!({ "name": "opendrobo_pool_status", "arguments": {} }),
                ),
            ],
            backend.clone(),
        );

        let tool = responses[0]["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .find(|tool| tool["name"] == "opendrobo_pool_status")
            .expect("pool tool");
        assert_eq!(tool["inputSchema"], empty_schema());
        assert_eq!(tool["annotations"]["readOnlyHint"], true);
        assert_eq!(tool["annotations"]["destructiveHint"], false);
        assert_eq!(tool["annotations"]["openWorldHint"], false);
        assert_eq!(responses[1]["result"]["resultType"], "complete");
        let calls = backend.calls.lock().expect("calls lock");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "opendrobo_pool_status");
        assert!(calls[0].1.is_empty());
    }

    #[test]
    fn cancelled_blocking_request_is_suppressed_while_later_input_is_handled() {
        let (input_sender, input_receiver) = mpsc::sync_channel(8);
        let (started_sender, started_receiver) = mpsc::channel();
        let (release_sender, release_receiver) = mpsc::channel();
        let (finished_sender, finished_receiver) = mpsc::channel();
        let (written_sender, written_receiver) = mpsc::channel();
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedWriter {
            bytes: bytes.clone(),
            complete: written_sender,
        };
        let backend = BlockingBackend {
            started: started_sender,
            release: release_receiver,
            finished: finished_sender,
        };
        let server = thread::spawn(move || {
            serve(
                ChannelReader {
                    receiver: input_receiver,
                    buffer: Vec::new(),
                    offset: 0,
                },
                &mut writer.clone(),
                backend,
            )
        });

        input_sender
            .send(
                format!(
                    "{}\n",
                    serde_json::to_string(&modern(
                        "tools/call",
                        19,
                        json!({ "name": "opendrobo_inventory", "arguments": {} }),
                    ))
                    .expect("request serializes"),
                )
                .into_bytes(),
            )
            .expect("call frame sent");
        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("backend started");
        input_sender
            .send(
                format!(
                    "{}\n",
                    serde_json::to_string(&json!({
                        "jsonrpc": "2.0",
                        "method": "notifications/cancelled",
                        "params": { "requestId": 19 }
                    }))
                    .expect("cancel serializes")
                )
                .into_bytes(),
            )
            .expect("cancel frame sent");
        input_sender
            .send(
                format!(
                    "{}\n",
                    serde_json::to_string(&modern("tools/list", 20, json!({})))
                        .expect("list serializes")
                )
                .into_bytes(),
            )
            .expect("later request sent");
        written_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("later response written");

        release_sender.send(()).expect("backend released");
        finished_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("backend finished");
        input_sender
            .send(
                format!(
                    "{}\n",
                    serde_json::to_string(&modern("tools/list", 21, json!({})))
                        .expect("second list serializes")
                )
                .into_bytes(),
            )
            .expect("post-cancel request sent");
        written_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("post-cancel response written");
        drop(input_sender);
        server
            .join()
            .expect("server thread joins")
            .expect("server exits");

        let responses = String::from_utf8(bytes.lock().expect("writer lock").clone())
            .expect("response is UTF-8")
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("response is JSON"))
            .collect::<Vec<_>>();
        assert_eq!(
            responses
                .iter()
                .map(|response| response["id"].clone())
                .collect::<Vec<_>>(),
            vec![json!(20), json!(21)]
        );
    }

    #[test]
    fn output_larger_than_frame_limit_but_smaller_than_output_limit_succeeds() {
        let payload = "x".repeat(MAX_FRAME_BYTES + 1);
        let calls = Arc::new(Mutex::new(0));
        let backend = SizedBackend {
            value: json!({ "payload": payload }),
            calls: calls.clone(),
        };
        let request = modern(
            "tools/call",
            9,
            json!({ "name": "opendrobo_inventory", "arguments": {} }),
        );
        let mut output = Vec::new();

        serve(
            Cursor::new(format!(
                "{}\n",
                serde_json::to_string(&request).expect("request serializes")
            )),
            &mut output,
            backend,
        )
        .expect("server exits at EOF");

        assert_eq!(*calls.lock().expect("calls lock"), 1);
        assert!(output.len() > MAX_FRAME_BYTES);
        assert!(output.len() <= MAX_OUTPUT_BYTES + 1);
        let response: Value = serde_json::from_slice(&output).expect("response is JSON");
        assert_eq!(response["id"], 9);
        assert_eq!(
            response["result"]["structuredContent"]["payload"]
                .as_str()
                .expect("payload string")
                .len(),
            MAX_FRAME_BYTES + 1
        );
    }

    #[test]
    fn output_larger_than_limit_returns_bounded_safe_error() {
        let marker = "do-not-leak";
        let calls = Arc::new(Mutex::new(0));
        let backend = SizedBackend {
            value: json!({ "secret": marker.repeat((MAX_OUTPUT_BYTES / marker.len()) + 1) }),
            calls: calls.clone(),
        };
        let request = modern(
            "tools/call",
            10,
            json!({ "name": "opendrobo_inventory", "arguments": {} }),
        );
        let mut output = Vec::new();

        serve(
            Cursor::new(format!(
                "{}\n",
                serde_json::to_string(&request).expect("request serializes")
            )),
            &mut output,
            backend,
        )
        .expect("server exits at EOF");

        assert_eq!(*calls.lock().expect("calls lock"), 1);
        assert!(output.len() <= MAX_OUTPUT_BYTES + 1);
        let wire = String::from_utf8(output).expect("response is UTF-8");
        assert!(!wire.contains(marker));
        let response: Value = serde_json::from_str(wire.trim_end()).expect("response is JSON");
        assert_eq!(response["id"], 10);
        assert_eq!(response["error"]["code"], -32603);
        assert_eq!(response["error"]["message"], "Response too large");
        assert!(response.get("result").is_none());
    }

    #[test]
    fn oversized_and_malformed_frames_are_bounded_and_recovery_is_safe() {
        let valid =
            serde_json::to_vec(&modern("tools/list", 7, json!({}))).expect("request serializes");
        let mut input = vec![b'x'; MAX_FRAME_BYTES + 1];
        input.push(b'\n');
        input.extend_from_slice(b"{not json}\n");
        input.extend_from_slice(&valid);
        input.push(b'\n');
        let mut output = Vec::new();
        let backend = MockBackend::default();

        serve(Cursor::new(input), &mut output, backend.clone()).expect("server exits at EOF");

        let responses = String::from_utf8(output)
            .expect("response is UTF-8")
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("response is JSON"))
            .collect::<Vec<_>>();
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0]["error"]["code"], -32600);
        assert_eq!(responses[1]["error"]["code"], -32700);
        assert_eq!(responses[2]["result"]["resultType"], "complete");
        assert!(backend.calls.lock().expect("calls lock").is_empty());
    }
}
