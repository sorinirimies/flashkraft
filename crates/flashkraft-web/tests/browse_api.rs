//! End-to-end checks of the guarded endpoints against a real server on a free
//! loopback port (static mode: no Topcoat bundle is needed).

use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use flashkraft_web::{
    app::{ServerOptions, serve},
    flash,
};

/// One HTTP/1.1 request → (status, body).
fn http(
    port: u16,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut req = format!("{method} {target} HTTP/1.1\r\nConnection: close\r\n");
    if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
        req += &format!("Host: 127.0.0.1:{port}\r\n");
    }
    for (k, v) in headers {
        req += &format!("{k}: {v}\r\n");
    }
    req += &format!("Content-Length: {}\r\n\r\n{body}", body.len());
    s.write_all(req.as_bytes()).unwrap();
    let mut out = String::new();
    let _ = s.read_to_string(&mut out);
    let status = out
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    (
        status,
        out.split("\r\n\r\n").nth(1).unwrap_or("").to_string(),
    )
}

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[test]
fn guarded_endpoints() {
    flash::configure(true);
    let token = flash::config().token.clone();
    let port = free_port();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.spawn(serve(ServerOptions {
        host: "127.0.0.1".into(),
        port,
        open_browser: false,
    }));
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.iso"), b"x").unwrap();
    std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
    let target = format!("/api/browse?path={}", dir.path().display());
    let tok = [("x-flashkraft-token", token.as_str())];

    // browse: allowed with token + loopback Host
    let (st, body) = http(port, "GET", &target, &tok, "");
    assert_eq!(st, 200, "{body}");
    assert!(body.contains("a.iso") && !body.contains("notes.txt"));

    // browse: refused without the token, with a wrong one, or with a foreign Host
    assert_eq!(http(port, "GET", &target, &[], "").0, 403);
    assert_eq!(
        http(port, "GET", &target, &[("x-flashkraft-token", "nope")], "").0,
        403
    );
    assert_eq!(
        http(port, "GET", &target, &[("Host", "evil.com"), tok[0]], "").0,
        403
    );

    // browse: a bad path is a clean 400, never a crash
    let (st, _) = http(
        port,
        "GET",
        "/api/browse?path=/definitely/not/here",
        &tok,
        "",
    );
    assert_eq!(st, 400);

    // flash start: guards come first
    let json = [("content-type", "application/json")];
    let req = r#"{"image":"/x.iso","devices":["/dev/sdb"],"confirm":1}"#;
    assert_eq!(http(port, "POST", "/api/flash", &json, req).0, 403);
    let both = [json[0], tok[0]];
    // right token, but the drive is not a detected one → rejected, not flashed
    assert_eq!(http(port, "POST", "/api/flash", &both, req).0, 400);
    // wrong confirmation count
    let bad = r#"{"image":"/x.iso","devices":["/dev/sdb"],"confirm":9}"#;
    assert_eq!(http(port, "POST", "/api/flash", &both, bad).0, 400);
    // cancel without token
    assert_eq!(http(port, "POST", "/api/flash/cancel", &json, "{}").0, 403);
    // nothing running
    let (st, body) = http(port, "GET", "/api/flash", &[], "");
    assert_eq!(st, 200);
    assert!(body.contains("\"running\":false"));
}
