//! An unknown command must be answered, not survived by luck.
//!
//! The control protocol's promise (see the crate docs) is that bad input
//! crossing the socket is an answerable error, never a panic: one request
//! line in, one well-formed response line out, the connection still usable
//! afterwards. This file pins that promise for the case the priority list
//! calls out — a method the table does not hold — over a real socket, on the
//! wire, so a framing regression cannot hide behind a mocked reply.

use serde_json::{json, Value};
use silica_control::{Client, Request, Response};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::Duration;

/// A socket path of our own. `tempfile` is not a dependency here, and a
/// socket has to live somewhere short anyway: the sun_path limit is 108
/// bytes. The pid keeps concurrent test binaries from colliding.
fn socket_path(name: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("silica-control-{}-{name}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    path
}

/// Answer requests the way the app does, for one connection, until the
/// client hangs up. Only the branches the protocol guarantees for bad input
/// are here: `ping` answers, an unparseable line gets one error line under
/// id 0 with the connection left open, and everything else is an error line
/// naming the method - as `handle`'s fallback arm does in the app.
fn serve(listener: silica_control::net::UnixListener) {
    let (stream, _) = listener.accept().expect("a connection");
    let reader = BufReader::new(stream.try_clone().expect("clone"));
    let mut writer = stream;
    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let Ok(request) = serde_json::from_str::<Request>(&line) else {
            let _ = writer.write_all(Response::err(0, "not a request").line().as_bytes());
            let _ = writer.flush();
            continue;
        };
        let response = match request.method.as_str() {
            "ping" => Response::ok(request.id, json!({"ok": true})),
            other => Response::err(request.id, format!("unknown method: {other}")),
        };
        if writer.write_all(response.line().as_bytes()).is_err() || writer.flush().is_err() {
            break;
        }
    }
}

/// One answer line, read whole: the framing is part of what is asserted.
fn answer(reader: &mut BufReader<silica_control::net::UnixStream>) -> (Response, String) {
    let mut line = String::new();
    reader.read_line(&mut line).expect("an answer line");
    let response: Response = serde_json::from_str(line.trim_end()).expect("parses as a response");
    (response, line)
}

#[test]
fn control_socket_rejects_unknown_command_gracefully() {
    let path = socket_path("unknown-command");
    let listener = silica_control::net::UnixListener::bind(&path).expect("binds");
    let server = std::thread::spawn(move || serve(listener));

    // Raw bytes on the wire: the unknown command is one request line, and
    // the answer is exactly one response line carrying an error — never a
    // panic, never a dropped connection.
    let stream = silica_control::net::UnixStream::connect(&path).expect("connects");
    let mut writer = stream.try_clone().expect("clone");
    let mut reader = BufReader::new(stream);

    writer
        .write_all(b"{\"id\":41,\"method\":\"frobnicate\",\"params\":{\"rows\":5}}\n")
        .expect("writes");
    let (response, line) = answer(&mut reader);
    assert!(
        line.ends_with('\n'),
        "the reply is one framed line: {line:?}"
    );
    assert_eq!(line.matches('\n').count(), 1, "{line:?}");
    assert_eq!(response.id, 41, "the same request id comes back");
    let error = response.error.expect("an error, not a result");
    assert!(
        error.contains("frobnicate"),
        "the method names itself: {error}"
    );
    assert_eq!(response.result, None, "no result rides along with an error");
    assert!(
        !line.contains("\"result\""),
        "one of the two keys, not both: {line}"
    );

    // A wrong-typed params object changes nothing about that contract.
    writer
        .write_all(b"{\"id\":42,\"method\":\"nonsense\",\"params\":\"oops\"}\n")
        .expect("writes");
    let (response, line) = answer(&mut reader);
    assert_eq!(line.matches('\n').count(), 1, "{line:?}");
    assert_eq!(response.id, 42);
    assert!(
        response
            .error
            .expect("an error")
            .contains("unknown method: nonsense"),
        "{line}"
    );
    assert_eq!(response.result, None);

    // The connection survives it: the next well-formed call still answers.
    writer
        .write_all(b"{\"id\":43,\"method\":\"ping\"}\n")
        .expect("writes");
    let (response, line) = answer(&mut reader);
    assert_eq!(response.id, 43);
    assert_eq!(response.result, Some(json!({"ok": true})), "{line}");
    assert_eq!(response.error, None);
    drop(writer);
    drop(reader);
    server.join().expect("the server thread");
    std::fs::remove_file(&path).ok();

    // The blocking client turns the same rejection into an Err message
    // rather than panicking, which is what the MCP server relies on. Its
    // call has no read timeout, so it gets a listener of its own: a peer
    // that never answers is a hang, not an error.
    let client_path = socket_path("unknown-command-client");
    let client_listener = silica_control::net::UnixListener::bind(&client_path).expect("binds");
    let client_server = std::thread::spawn(move || serve(client_listener));
    let mut client = Client::connect(&client_path, Duration::from_secs(2)).expect("connects");
    let e = client
        .call("frobnicate", Value::Null)
        .expect_err("rejected, not panicked");
    assert!(e.contains("unknown method"), "{e}");
    assert!(e.contains("frobnicate"), "{e}");

    drop(client);
    client_server.join().expect("the server thread");
    std::fs::remove_file(&client_path).expect("removes the socket");
}

/// A line that is not a request at all is still answered (under id 0) and
/// leaves the connection open, so a client that mistypes one call carries
/// on with the next.
#[test]
fn a_malformed_line_is_answered_and_forgotten() {
    let path = socket_path("malformed-line");
    let listener = silica_control::net::UnixListener::bind(&path).expect("binds");
    let server = std::thread::spawn(move || serve(listener));

    let stream = silica_control::net::UnixStream::connect(&path).expect("connects");
    let mut writer = stream.try_clone().expect("clone");
    let mut reader = BufReader::new(stream);

    writer.write_all(b"this is not json\n").expect("writes");
    let (response, line) = answer(&mut reader);
    assert_eq!(line.matches('\n').count(), 1, "{line:?}");
    assert_eq!(response.id, 0, "no request could supply an id");
    assert!(response.error.is_some(), "{line}");
    assert_eq!(response.result, None);

    // One bad line does not end the session.
    writer
        .write_all(b"{\"id\":1,\"method\":\"ping\"}\n")
        .expect("writes");
    let (response, line) = answer(&mut reader);
    assert_eq!(response.id, 1, "{line}");
    assert_eq!(response.result, Some(json!({"ok": true})));
    drop(writer);
    drop(reader);

    server.join().expect("the server thread");
    std::fs::remove_file(&path).expect("removes the socket");
}
