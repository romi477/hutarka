use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn answer(text: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "content": [{"type": "text", "text": text}],
        "stop_reason": "end_turn"
    }))
}

fn hutarka(server: &MockServer) -> Command {
    let mut cmd = Command::cargo_bin("hutarka").unwrap();
    cmd.env("HUTARKA_API_KEY", "test-key")
        .env("HUTARKA_BASE_URL", server.uri());
    cmd
}

/// Runs the binary off the async runtime so the mock server keeps serving.
async fn run(mut cmd: Command) -> assert_cmd::assert::Assert {
    tokio::task::spawn_blocking(move || cmd.assert())
        .await
        .unwrap()
}

async fn requests(server: &MockServer) -> Vec<Value> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| serde_json::from_slice(&r.body).unwrap())
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn to_translates_from_argument() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(json!({
            "system": "Translate the text to English. Output only the translation.",
            "messages": [{"role": "user", "content": "Привет"}]
        })))
        .respond_with(answer("Hello"))
        .expect(1)
        .mount(&server)
        .await;
    let mut cmd = hutarka(&server);
    cmd.args(["--to", "en", "Привет"]);
    run(cmd).await.success().stdout("Hello");
}

#[tokio::test(flavor = "multi_thread")]
async fn prompt_and_stdin() {
    let server = MockServer::start().await;
    Mock::given(body_partial_json(json!({
        "system": "Fix spelling.",
        "messages": [{"role": "user", "content": "тэкст\nз радкамі\n"}]
    })))
    .respond_with(answer("fixed"))
    .expect(1)
    .mount(&server)
    .await;
    let mut cmd = hutarka(&server);
    cmd.args(["--prompt", "Fix spelling."])
        .write_stdin("тэкст\nз радкамі\n");
    run(cmd).await.success().stdout("fixed");
}

#[tokio::test(flavor = "multi_thread")]
async fn model_and_effort_flags_are_sent() {
    let server = MockServer::start().await;
    Mock::given(body_partial_json(json!({
        "model": "claude-sonnet-5-5",
        "output_config": {"effort": "high"}
    })))
    .respond_with(answer("ok"))
    .expect(1)
    .mount(&server)
    .await;
    let mut cmd = hutarka(&server);
    cmd.args([
        "--model",
        "claude-sonnet-5-5",
        "--effort",
        "high",
        "--to",
        "en",
        "t",
    ]);
    run(cmd).await.success().stdout("ok");
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_key_exits_1() {
    let server = MockServer::start().await;
    let mut cmd = hutarka(&server);
    cmd.env_remove("HUTARKA_API_KEY").args(["--to", "en", "t"]);
    run(cmd)
        .await
        .code(1)
        .stderr(predicate::str::contains("HUTARKA_API_KEY is not set"));
    assert!(requests(&server).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_input_exits_2() {
    let server = MockServer::start().await;
    let mut cmd = hutarka(&server);
    cmd.args(["--to", "en"]).write_stdin("   \n");
    run(cmd).await.code(2);
    assert!(requests(&server).await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn api_error_exits_1_with_message() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "type": "error",
            "error": {"type": "authentication_error", "message": "invalid x-api-key"}
        })))
        .mount(&server)
        .await;
    let mut cmd = hutarka(&server);
    cmd.args(["--to", "en", "t"]);
    run(cmd)
        .await
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("API error 401: invalid x-api-key"));
}

#[tokio::test(flavor = "multi_thread")]
async fn to_and_prompt_together_exit_2() {
    let server = MockServer::start().await;
    let mut cmd = hutarka(&server);
    cmd.args(["--to", "en", "--prompt", "x", "t"]);
    run(cmd).await.code(2);
}

#[tokio::test(flavor = "multi_thread")]
async fn key_with_trailing_newline_is_trimmed() {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::header("x-api-key", "test-key"))
        .respond_with(answer("ok"))
        .expect(1)
        .mount(&server)
        .await;
    let mut cmd = hutarka(&server);
    cmd.env("HUTARKA_API_KEY", "test-key\n")
        .args(["--to", "en", "t"]);
    run(cmd).await.success().stdout("ok");
}
