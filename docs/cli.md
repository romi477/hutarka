# `hutarka` command-line tool

`hutarka` sends a text to an LLM (Anthropic API) together with a prompt you give it, and prints the answer. Use it to translate or rewrite text from the terminal, in pipes and in scripts.

It is a standalone program. It does not need the Hutarka app, does not run in the background, and reads no app files. Each call is one short process: it sends one request, prints the answer and exits.

## Install

From the repository root:

```bash
cargo install --path crates/hutarka-cli
```

This builds a release binary and copies it to `~/.cargo/bin/hutarka`. Nothing else is registered in the system: no service, no login item. `~/.cargo/bin` must be on your `PATH` (rustup sets this up).

After changing the source, run the same command again: the installed file is a copy and is not updated by itself.

Uninstall:

```bash
cargo uninstall hutarka-cli
```

## API key

`hutarka` reads the key from the `HUTARKA_API_KEY` environment variable. Add it to your shell profile so every new terminal has it:

```bash
echo 'export HUTARKA_API_KEY="sk-ant-..."' >> ~/.zshrc
```

There is deliberately no `--key` flag: a key on the command line would end up in shell history and in the process list. Leading and trailing whitespace in the variable is ignored.

## Usage

```
hutarka (--prompt <PROMPT> | --to <LANG>) [--model <MODEL>] [--effort <EFFORT>] [TEXT]
```

The input text is the `TEXT` argument or, when it is omitted, everything read from stdin. The answer is printed to stdout and nothing else is: no labels, no formatting.

Exactly one of `--prompt` and `--to` is required.

### `--prompt <PROMPT>`

The system prompt, sent exactly as you wrote it. `hutarka` adds nothing to it.

```bash
hutarka --prompt "Translate to English. IT context. Output only the translation." "Нужно поднять версию зависимости"
pbpaste | hutarka --prompt "Correct spelling errors in this Russian text. Output only the text."
hutarka --prompt "Summarize in three bullet points." < notes.txt
```

### `--to <LANG>`

A plain, literal translation. `LANG` is a short code (`en`, `ru`, `be`, `pl`) or any language name (`German`, `Ukrainian`, ...). This is the only built-in prompt in `hutarka`, and it is sent verbatim:

```
Translate the text to <LANG>. Output only the translation.
```

The codes expand to `English`, `Russian`, `Belarusian`, `Polish`; any other value is used as written.

```bash
hutarka --to en "Добры дзень, як справы?"     # Good day, how are you?
hutarka --to be "Good morning"                 # Добрай раніцы
pbpaste | hutarka --to ru | pbcopy             # translate the clipboard in place
```

For anything beyond a literal translation (domain, tone, terms to keep), write your own `--prompt`.

### `--model <MODEL>`

Model id. Default: `claude-haiku-5-5` (fast and cheap, good for translation and cleanup). Any Anthropic model id works, for example `claude-sonnet-5-5` or `claude-opus-5-5`.

### `--effort <low|medium|high>`

How much the model thinks before answering. Default: `low`, which is the fastest. Raise it for harder tasks; it costs more time and tokens.

## Output

- Only the model's text goes to stdout. Errors go to stderr, prefixed with `hutarka:`.
- A trailing newline is added only when stdout is a terminal. In a pipe the text is passed on exactly, so `| pbcopy` does not add an empty line.
- Input is sent unchanged: line breaks, Cyrillic and emoji are preserved.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success, the answer was printed. |
| `1` | The request failed: missing key, network error, API error, the model declined, or the answer stopped before it was finished. Nothing is printed to stdout. |
| `2` | Usage or input error: wrong flags, no input, or input that is only whitespace. No request is sent. |

A partial answer is never printed as success: if the model stops for any reason other than finishing normally, `hutarka` exits `1`.

## Errors

| Message | What to do |
|---|---|
| `HUTARKA_API_KEY is not set` | Set the variable (see [API key](#api-key)); open a new terminal if you added it to `~/.zshrc`. |
| `no input: pass text as an argument or pipe it on stdin` | You ran `hutarka` with no text in an interactive terminal. Pass text or pipe it. |
| `input is empty` | The text was empty or only whitespace. |
| `API error 401: ...` | The key is wrong or revoked. |
| `API error 429: ...` / `API error 529: ...` | Rate limit or the API is overloaded; retry later. |
| `network error: ...` | No connection, DNS failure or timeout; the message names the cause. |
| `the model declined the request` | The model refused to answer this input. |
| `the answer was cut off (max_tokens)` | The answer hit the output limit (16000 tokens). Split the input. |
| `the answer stopped early (...)` | The model stopped for another reason, named in brackets (for example the input was too large). |

## Timing and cost

With the defaults (Haiku 5.5, effort `low`) a short translation takes about 0.7–0.9 s including process start. Each call is billed by Anthropic per token; a typical dictation-sized translation costs a small fraction of a cent.

## Shell aliases

`hutarka` stores no prompts. Keep the ones you reuse as aliases or functions in `~/.zshrc`:

```bash
alias ten='hutarka --to en'
alias tru='hutarka --to ru'
fixru() { hutarka --prompt "Correct spelling errors in this Russian text. Output only the text." "$@"; }
```
