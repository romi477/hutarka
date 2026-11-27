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
