use hutarka_llm::{Client, Error, Request, build_body};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn answer(text: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "content": [{"type": "text", "text": text}],
        "stop_reason": "end_turn"
    }))
}

fn client(server: &MockServer) -> Client {
    Client::new("test-key".into(), server.uri()).expect("client")
}

#[tokio::test]
async fn sends_required_headers_and_body() {
    let server = MockServer::start().await;
    let req = Request::new("claude-haiku-5-5", "SYS", "hello");
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2026-10-21"))
        .and(body_json(build_body(&req)))
        .respond_with(answer("Hi"))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(client(&server).complete(&req).await, Ok("Hi".to_string()));
}

#[tokio::test]
async fn sends_unicode_multiline_text_unchanged() {
    let server = MockServer::start().await;
    let text = "Прывітанне, ўсім!\n\nДругі радок 🙂";
    let reply = "Hello, everyone!\n\nSecond line 🙂";
    let req = Request::new("m", "s", text);
    Mock::given(body_json(build_body(&req)))
        .respond_with(answer(reply))
        .expect(1)
        .mount(&server)
        .await;
    assert_eq!(client(&server).complete(&req).await, Ok(reply.to_string()));
}

#[tokio::test]
async fn api_error_maps_status_and_message() {
    for (status, message) in [(401, "invalid x-api-key"), (529, "Overloaded")] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(status).set_body_json(json!({
                "type": "error",
                "error": {"type": "some_error", "message": message}
            })))
            .mount(&server)
            .await;
        let got = client(&server).complete(&Request::new("m", "s", "t")).await;
        assert_eq!(
            got,
            Err(Error::Api {
                status,
                message: message.to_string()
            })
        );
    }
}

#[tokio::test]
async fn api_error_without_json_body_keeps_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(502).set_body_string("bad gateway"))
        .mount(&server)
        .await;
    let got = client(&server).complete(&Request::new("m", "s", "t")).await;
    assert!(
        matches!(got, Err(Error::Api { status: 502, .. })),
        "{got:?}"
    );
}

#[tokio::test]
async fn network_failure_names_its_cause() {
    let client = Client::new("k".into(), "http://127.0.0.1:9".into()).expect("client");
    let got = client.complete(&Request::new("m", "s", "t")).await;
    match got {
        Err(Error::Network(msg)) => assert!(msg.to_lowercase().contains("refused"), "{msg}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn html_error_body_becomes_one_line_reason() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(502).set_body_string(
            "<html>\n<head><title>502</title></head>\n<body>bad</body>\n</html>\n",
        ))
        .mount(&server)
        .await;
    let got = client(&server).complete(&Request::new("m", "s", "t")).await;
    assert_eq!(
        got,
        Err(Error::Api {
            status: 502,
            message: "Bad Gateway".into()
        })
    );
}

#[tokio::test]
async fn empty_error_body_uses_reason() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    let got = client(&server).complete(&Request::new("m", "s", "t")).await;
    assert_eq!(
        got,
        Err(Error::Api {
            status: 503,
            message: "Service Unavailable".into()
        })
    );
}

#[tokio::test]
async fn plain_text_error_body_is_collapsed_and_capped() {
    let server = MockServer::start().await;
    let body = format!("upstream\n  failed {}", "x".repeat(500));
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string(body))
        .mount(&server)
        .await;
    match client(&server).complete(&Request::new("m", "s", "t")).await {
        Err(Error::Api {
            status: 500,
            message,
        }) => {
            assert!(message.starts_with("upstream failed x"), "{message}");
            assert!(!message.contains('\n'));
            assert!(message.chars().count() <= 201, "{}", message.len());
        }
        other => panic!("{other:?}"),
    }
}
