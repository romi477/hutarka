# Hutarka documentation

Hutarka is a private, local-first voice and translation tool for the Mac (Apple Silicon). See the [project README](../README.md) for what it is meant to become.

This folder documents what exists and works today. Plans and design notes are not here.

## What works today

| Component | State | Docs |
|---|---|---|
| `hutarka` CLI: send text to an LLM with an explicit prompt, or translate it | available | [cli.md](cli.md) |
| Hutarka app: dictation modes, selection translator, meeting capture | not built yet | — |

## Requirements

- macOS on Apple Silicon.
- Rust toolchain (stable) to build from source: [rustup.rs](https://rustup.rs).
- An Anthropic API key, for anything that calls an LLM.

## Building from source

```bash
git clone git@github.com:romi477/hutarka.git
cd hutarka
cargo test --workspace
```

The repository is a Cargo workspace:

| Crate | What it is |
|---|---|
| `crates/hutarka-llm` | Library: sends a prompt and a text to the Anthropic Messages API and returns the answer. Contains no prompts of its own. |
| `crates/hutarka-cli` | The `hutarka` command-line tool, built on `hutarka-llm`. |
