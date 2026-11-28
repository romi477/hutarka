use hutarka_llm::{Client, Error, Request};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request as WireRequest, ResponseTemplate};

fn event(name: &str, data: serde_json::Value) -> String {
    format!("event: {name}\ndata: {data}\n\n")
}

fn delta(text: &str) -> String {
    event(
        "content_block_delta",
        json!({"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": text}}),
    )
}

fn stop(reason: &str) -> String {
    event(
        "message_delta",
        json!({"type": "message_delta", "delta": {"stop_reason": reason}}),
    )
}

fn sse(body: String) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.into_bytes(), "text/event-stream")
}

fn client(server: &MockServer) -> Client {
    Client::new("test-key".into(), server.uri()).expect("client")
}

async fn run(server: &MockServer) -> (Result<String, Error>, Vec<String>) {
    let mut seen = Vec::new();
    let result = client(server)
        .stream(&Request::new("m", "s", "t"), |t| seen.push(t.to_string()))
        .await;
    (result, seen)
}

#[tokio::test]
async fn stream_collects_deltas() {
    let server = MockServer::start().await;
    let body = [
        event(
            "message_start",
            json!({"type": "message_start", "message": {}}),
        ),
        delta("Добры "),
        delta("дзень"),
        stop("end_turn"),
        event("message_stop", json!({"type": "message_stop"})),
    ]
    .concat();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(|req: &WireRequest| {
            serde_json::from_slice::<serde_json::Value>(&req.body)
                .map(|b| b["stream"] == json!(true))
                .unwrap_or(false)
        })
        .respond_with(sse(body))
        .expect(1)
        .mount(&server)
        .await;
    let (result, seen) = run(&server).await;
    assert_eq!(result, Ok("Добры дзень".to_string()));
    assert_eq!(seen, vec!["Добры ", "дзень"]);
}

#[tokio::test]
async fn stream_maps_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_json(
            json!({"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}}),
        ))
        .mount(&server)
        .await;
    let (result, seen) = run(&server).await;
    assert_eq!(
        result,
        Err(Error::Api {
            status: 401,
            message: "invalid x-api-key".into()
        })
    );
    assert!(seen.is_empty());
}

#[tokio::test]
async fn stream_error_event() {
    let server = MockServer::start().await;
    let body = [
        delta("Half"),
        event(
            "error",
            json!({"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}),
        ),
    ]
    .concat();
    Mock::given(method("POST"))
        .respond_with(sse(body))
        .mount(&server)
        .await;
    let (result, _) = run(&server).await;
    assert_eq!(result, Err(Error::Stream("Overloaded".into())));
}

#[tokio::test]
async fn stream_truncated() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse([delta("partial"), stop("max_tokens")].concat()))
        .mount(&server)
        .await;
    assert_eq!(run(&server).await.0, Err(Error::Truncated));
}

#[tokio::test]
async fn stream_refusal() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse(stop("refusal")))
        .mount(&server)
        .await;
    assert_eq!(run(&server).await.0, Err(Error::Refusal));
}

#[tokio::test]
async fn stream_without_stop_is_stopped() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(sse(delta("cut")))
        .mount(&server)
        .await;
    assert_eq!(run(&server).await.0, Err(Error::Stopped("none".into())));
}

#[tokio::test]
async fn malformed_event_fails_the_answer() {
    let server = MockServer::start().await;
    let body = [
        delta("Hello "),
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"te\n\n".to_string(),
        stop("end_turn"),
    ]
    .concat();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(sse(body))
        .mount(&server)
        .await;
    let (result, _) = run(&server).await;
    assert!(matches!(result, Err(Error::Stream(_))), "{result:?}");
}

#[tokio::test]
async fn keepalive_comment_is_ignored() {
    let server = MockServer::start().await;
    let body = [
        ": keepalive\n\n".to_string(),
        delta("ok"),
        stop("end_turn"),
    ]
    .concat();
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(sse(body))
        .mount(&server)
        .await;
    let (result, _) = run(&server).await;
    assert_eq!(result.expect("answer"), "ok");
}

/// A server that sends a whole answer and then keeps the connection open.
#[tokio::test]
async fn answer_ends_at_message_stop_without_waiting_for_close() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 8192];
        let _ = sock.read(&mut buf).await;
        let body = [
            delta("done"),
            stop("end_turn"),
            event("message_stop", json!({"type": "message_stop"})),
        ]
        .concat();
        let head = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n";
        let chunk = format!("{:x}\r\n{body}\r\n", body.len());
        sock.write_all(head.as_bytes()).await.expect("head");
        sock.write_all(chunk.as_bytes()).await.expect("chunk");
        // Never sends the final chunk.
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    });
    let client = Client::new("k".into(), format!("http://{addr}")).expect("client");
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        client.stream(&Request::new("m", "s", "t"), |_| {}),
    )
    .await
    .expect("returned before the connection closed");
    assert_eq!(result.expect("answer"), "done");
}
