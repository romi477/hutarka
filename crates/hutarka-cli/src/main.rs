//! `hutarka`: send text to an LLM with an explicit prompt and print the answer.
//! Standalone: never talks to the Hutarka app.

mod args;
mod input;

use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser;
use hutarka_llm::{Client, Request};

use args::{Args, system_prompt};
use input::read_input;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args = Args::parse();
    let stdin = std::io::stdin();
    let text = match read_input(args.text.clone(), stdin.is_terminal(), stdin.lock()) {
        Ok(text) => text,
        Err(e) => {
            eprintln!("hutarka: {e}");
            return ExitCode::from(2);
        }
    };

    let client = match Client::from_env() {
        Ok(client) => client,
        Err(e) => {
            eprintln!("hutarka: {e}");
            return ExitCode::from(1);
        }
    };

    let mut request = Request::new(args.model.clone(), system_prompt(&args), text);
    request.effort = args.effort.into();

    match client.complete(&request).await {
        Ok(answer) => {
            let mut out = std::io::stdout().lock();
            let newline = if std::io::stdout().is_terminal() {
                "\n"
            } else {
                ""
            };
            if let Err(e) = write!(out, "{answer}{newline}").and_then(|_| out.flush()) {
                // A closed pipe (`| head`) is normal; anything else (full disk) is said.
                if e.kind() != std::io::ErrorKind::BrokenPipe {
                    eprintln!("hutarka: cannot write the answer: {e}");
                }
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("hutarka: {e}");
            ExitCode::from(1)
        }
    }
}
