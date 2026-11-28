# AGENTS.md

Guidance for AI coding assistants working in this repository. See `README.md` for what Hutarka is meant to do.

## Status

Step 01 done: the `hutarka` CLI and the `hutarka-llm` crate (`crates/`). Step 02 done: the app (`app/`) with the selection translator: menu bar icon, popup, Settings (General with hotkey and key, Translation with target languages and prompts, empty Audio capture and Speech), lookup journal. No dictation or meetings yet.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
(cd app && npm run build)                 # type-check and build the UI
scripts/make-signing-cert.sh              # once per Mac: "Hutarka Local" signing identity
scripts/app.sh install                    # build, install ~/Applications/Hutarka.app, start it
scripts/app.sh cli                        # installs ~/.cargo/bin/hutarka
```

App log: `~/Library/Logs/dev.hutarka.app/Hutarka.log`.

Work goes in steps, each planned and closed in `docs_/steps/` (see `docs_/steps/README.md`).

Documentation layout: `docs/` is the official user documentation of what works today (`docs/README.md` index, `docs/cli.md`, ...). Any change to user-visible behavior updates the matching page in the same commit; nothing planned or unbuilt goes there. Everything intermediate (research, step plans, notes) lives in `docs_/`, which is git-ignored and never committed.

## Who it is for

One user, one machine: the author's Apple Silicon MacBook. Hutarka is **not distributed**. Never add anything whose only purpose is distribution: notarization, auto-update, installers, release CI, telemetry, crash reporting, onboarding for strangers.

## Decisions

- **Platform**: macOS 14.4+ on Apple Silicon only. No Windows, no Linux. Delete cross-platform branches (`cfg(windows)`, `cfg(target_os = "linux")`, X11/Wayland fallbacks) instead of keeping them compiling.
- **Language**: Rust for all app logic. TypeScript only for the settings UI.
- **Layout**: Cargo workspace: `crates/hutarka-llm` (LLM core), `crates/hutarka-cli` (the `hutarka` binary), `app/src-tauri` (the Tauri app, package `hutarka-app`) with its React UI in `app/src`. The Swift `yogurt-audio` helper will sit beside them. Frontend tooling is npm.
- **Base**: a fresh Tauri 2 app that copies pieces of [Handy](https://github.com/cjpais/Handy) (Tauri 2, Rust backend, React frontend) as each step needs them (tray, panel, later shortcuts, ASR, paste), instead of importing all of Handy and cutting it down. Not a fork; we do not track upstream.
- **UI**: React + Tailwind from Handy, kept small. **English only**: remove i18next and translation files, write UI strings inline.
- **No auto-update**: remove `tauri-plugin-updater` and the update checker.
- **System audio capture**: Yogurt's `yogurt-audio` Swift helper used as a sidecar process, unchanged at first. Built with `swiftc` from the Command Line Tools (no Xcode project needed). Porting it to Rust (`objc2-*` crates) is a later, optional step.
- **MCP server**: in Rust, local only, read-only, off by default. Ported from Yogurt's `app/src/main/mcp/server.ts`, plus a live `get_lines_since` tool reading the JSONL journal (see Storage).
- **License**: undecided (both sources are MIT; MIT is the likely choice).
- **No subscriptions by default**: Hutarka replaces a paid dictation app, so the default setup costs nothing beyond pay-per-use API calls. Speech recognition is **local by default**. A paid cloud ASR subscription is acceptable later if Whisper quality is not good enough (see "Speech engines").
- **API keys**: stored in the app's settings (entered in the Settings window). Settings live in the app data directory, **never** in the open `~/Hutarka` folder, so keys are not synced or shared with the journals.

## Non-goals

Do not build these, even as options:

- Automatic meeting detection, recording prompts when a call app starts, auto-stop. Recording is always started and stopped by hand.
- Calendar integration (titles, attendees).
- Automatic meeting summaries. Summaries are done on demand, by asking an AI client over MCP.
- Anything visible to other call participants: no bot, no announcement. Muting the mic in the call app while dictating is the author's job, not Hutarka's.

## Speech engines

Speech recognition sits behind one engine interface (audio in, segments with timestamps out, batch and streaming), so engines can be swapped per mode and per meeting track.

- **Local, primary**: Handy's `transcribe-cpp` (GGUF models) on Apple Silicon GPU via Metal. The model is chosen per language/mode, not one model for everything.

Handy's catalog (`src-tauri/src/catalog/catalog.json`) carries benchmark word error rates (FLEURS test set, Q8_0, speed = times faster than real time on a Ryzen laptop with Vulkan; real speech and Apple Silicon will differ):

| Model | ru | be | pl | en | speed |
|---|---|---|---|---|---|
| Whisper Large v3 | 4.96 | 43.78 | 4.69 | 4.03 | 2x |
| GigaAM v3 E2E-RNN-T (Sber, MIT, Russian only, 221M) | 5.35 | — | — | — | 15x |
| GigaAM v3 E2E-CTC | 5.53 | — | — | — | 35x |
| Parakeet Ultra | 5.63 | — | 6.16 | 4.26 | 23x |
| Whisper Large v3 Turbo | 5.93 | 50.65 | 5.81 | 4.38 | 3x |

Conclusions to verify on the author's own voice:

- **Russian**: GigaAM v3 E2E (already in Handy's catalog, ~270 MB) is nearly as accurate as Whisper Large v3 and 7-17x faster: the likely default for Russian dictation.
- **Russian + Polish + English in one model**: Parakeet Ultra is a fast candidate.
- **Belarusian is the weak spot**: no catalog model lists it except Whisper, and Whisper's error rate is 44-51%. Options to test: ElevenLabs Scribe (check its Belarusian support), or recognize with a Russian model and let the LLM restore Belarusian text. Belarusian is the most likely reason to need ElevenLabs.
- **Apple SpeechAnalyzer** (`SpeechTranscriber`, Neural Engine, free, on-device): on this Mac (macOS 27.2) it supports English but **not Russian, Belarusian or Polish** (checked with `SpeechTranscriber.supportedLocales` on 2026-11-21). Useful only for English modes and English meetings; re-check the locale list after macOS updates.
- **Cloud: ElevenLabs, designed in from the start.** Whisper quality (especially Belarusian) is the main technical risk, and ElevenLabs Scribe with the author's own key (possibly a paid subscription) is the planned answer. The client may be implemented later, but the engine interface is designed against both Whisper and ElevenLabs from day one: batch upload of a file, realtime streaming over WebSocket, network latency and failures, per-request cost, language hints. Engine choice is per mode and per meeting (live vs. after-meeting pass can use different engines), and a missing key or no network falls back to Whisper.
- **Not used for translation**: Whisper's built-in translate-to-English (`translate_to_english` in Handy) is too low quality. All translation goes through the LLM step.

## Modes

The core dictation feature, modeled on Spokenly. A mode is:

- a trigger: a key and a gesture. **Double-tap of a single key** (e.g. a modifier such as right Option) is required; hold and toggle are also supported. Handy has no double-tap; it is added in the shortcut layer.
- a speech language (or auto-detect) and a speech model;
- an optional LLM step: provider, model, prompt;
- output: paste into the focused app.

Required modes from day one: **translate to English**, **translate to Russian**, **translate to Belarusian**, and **clean dictation** (same language, corrected). A raw mode without LLM stays available as a fallback (no network, no key).

The author's current Spokenly setup, to reproduce first (mode: trigger, instruction):

- **English (default)**: Right ⌘. "Translate the dictated text to English (IT field)." The instruction carries domain context.
- **Plain Russian**: Right ⌥. "The text will be in Russian; do nothing, only correct spelling errors." (= clean dictation)
- **Belarusian**: Fn. Instruction written in Belarusian: the output must be in Belarusian.

Implications for the shortcut layer:

- **Single modifier keys as triggers, side-specific**: Right ⌘ and Right ⌥ must not fire on Left ⌘ / Left ⌥, and a tap must not fire when the key is used in a combo (⌘C). Handy's `handy-keys` names modifiers without a side (`command`, `option`, `fn`); left/right distinction has to be added. Fn is already supported.
- A mode's instruction is free text written by the user in any language; Hutarka wraps it in a fixed system prompt (correct recognition errors, output only the result).
- **Latency target**: Spokenly's "Fast" cleanup is under 1 s after release; its "Smart" translation is ~4 s. Hutarka aims for under 1 s for cleanup and translation with Haiku 5.5; measure it.
- **Vocabulary matters**: Spokenly turned "Hutarka" into "hudorko". The user vocabulary (names, product names, terms) goes into the ASR prompt (Whisper initial prompt) and the LLM prompt.

**Every mode corrects the recognition by default.** Raw ASR output has wrong letters, wrong word endings, misheard names and weak punctuation. The LLM step fixes these, the way Spokenly does:

- Clean dictation: one call, prompt "fix recognition errors, endings, punctuation; do not rephrase or add anything".
- Translation modes: one call that corrects and translates together (no separate cleanup call, to keep latency and cost down).
- The prompt receives context that helps correction: the speech language, the user's vocabulary (names, terms; Handy's custom words list), and optionally the focused app's name.
- Meetings: live lines stay raw (speed matters, and they are rewritten later); correction runs in the after-meeting "better pass", in batches of lines, and writes its own journal file.

Translation goes through an LLM API with the author's own key, **Anthropic API by default** (Handy's `llm_client.rs` already speaks the Messages format). Model is per mode, Claude Haiku 5.5 (`claude-haiku-5-5`) is the expected default for translation: cheap and fast. Keep the request minimal for latency (short system prompt, low effort or thinking off where the model allows it). OpenAI-compatible endpoints (Ollama, etc.) stay supported for fully offline use.

## LLM core and CLI

Dictation modes, the selection translator and meeting correction all do the same thing: text in, text out through an LLM. The request itself lives in **one Rust library crate** (working name `hutarka-llm`): providers (Anthropic first, OpenAI-compatible), API key lookup, streaming, errors and timeouts.

**No prompts are hard-coded in the core, and the CLI has exactly one, documented below (`--to`).** The core is pure transport: it sends exactly the prompt, model and text it is given. Every instruction (translate to English, clean up Russian, dictionary entry for a word) is data:

- in the app: stored in Settings (each mode's prompt, the selection translator's prompts), editable, with sensible starting values written into the default settings file, not into code;
- in the CLI: passed explicitly as flags on every call.

**CLI binary** (`hutarka`), a small standalone binary. It works whether or not the app is running: it never talks to the app, needs no daemon, and reads nothing from the app's files. Each call is one short process (request, answer, exit):

```
hutarka --prompt "Translate to English. IT field. Output only the translation." "текст"
pbpaste | hutarka --prompt "Correct spelling errors in this Russian text. Output only the text."
hutarka --model claude-sonnet-5-5 --prompt "..." < notes.txt
hutarka --to en "текст"                  # plain literal translation
```

- `--prompt`: the system prompt, sent as is. Input text: the positional argument, or stdin.
- `--to <lang>`: plain one-to-one translation with no extra processing. It sends one fixed minimal prompt, printed verbatim by `hutarka --help` so nothing is hidden: `Translate the text to <lang>. Output only the translation.` (lang: `en`, `ru`, `be`, `pl` or any language name). `--to` and `--prompt` are mutually exclusive; exactly one is required.
- `--model` (optional, default `claude-haiku-5-5`). No other hidden settings.
- Key: `HUTARKA_API_KEY` environment variable. No `--key` flag: keys on the command line end up in shell history and `ps`.
- `HUTARKA_BASE_URL` (optional, documented in `docs/cli.md`): another Anthropic-compatible endpoint; the tests use it for a mock server. The app always uses the public API.
- Output is the model's text only, so it composes in pipes and shell aliases (an alias or script is where a reusable prompt lives on the shell side).
- Built first, before the app: it is tiny and immediately tests prompt quality and Haiku latency on real text.

## Selection translator

A replacement for Reverso Context: select text anywhere, press a hotkey, a popup shows the translation.

- **Getting the selection**: Accessibility API (`AXSelectedText` of the focused element) first; fallback: save clipboard, send ⌘C, read, restore (Handy's clipboard code). Works in native apps, PDFs, browsers (fallback path).
- **Word vs. text**: a single word or short phrase gets a dictionary-style answer (meanings with part of speech, a few example sentences, common collocations), longer text gets a plain translation. Decided by length; two prompts, both stored in Settings.
- **Target languages**: cards in Settings › Translation, chosen from a fixed list in code (English, Russian, Belarusian, Polish, German, Spanish, Greek, Italian, French). Each card has its own translation prompt; one card is the default.
- **Source language**: never detected by Hutarka; the model works it out. Every lookup starts with the default target; the popup's target button (menu of the cards, ⌘1…⌘9) switches it for that lookup only. Translating into the language the text is already in is not guarded against: the author switches by hand.
- **Explain**: a checkbox in the popup (⌘E), remembered between lookups. On, the answer is a deep explanation instead of a plain translation: for a word, meanings plus several common sentences using it, each with a translation, to learn how it is used in context; for a phrase, its idioms and grammar. One more prompt, stored in Settings.
- **Prompts are editable in Settings › Translation**: one translation prompt per target card (new cards get one from a template), plus the word-lookup prompt and the Explain prompt, which take a `{to}` placeholder (`{from}` in older edited prompts renders as "the original language"). Starting values live in the default settings file. The popup's language menu has "Edit prompts…", which opens that pane.
- **Popup**: variant A of the mockups. Small non-activating panel near the mouse, does not steal focus; header with the target-language button and model/latency; streams the answer as it arrives; footer with Copy (⌘C), Replace (↵: paste the translation over the selection in editable fields) and the Explain checkbox; Esc or click outside closes it. Follows the system light/dark theme.
- **Hotkey**: its own trigger in the shortcut layer, same options as modes (key combo, single key, double-tap).
- Results are written to the History journal (`kind: lookup`) so looked-up words can be reviewed later.

## App surfaces

- **Menu bar icon** (Handy's `tray.rs`), the main entry point. Shows state (idle / dictating / recording a meeting). Quick menu:
  - Paste last text, Copy last text (both: last result after LLM; a submenu offers the raw ASR text too). Handy already has "copy last transcript".
  - Recent results: the last few dictations/translations, click to copy.
  - **Record meeting** submenu with two choices: "Audio only" and "Live transcript" (see Meetings); Stop. The current meeting shows its title, duration and mode.
  - **Transcribe file**: pick an audio/video file, get a transcript in History (Handy already decodes files via symphonia).
  - Current mode list (which key triggers what), at a glance.
  - Translate (the selection, or an empty field when nothing is selected), Open History, Settings, Quit.
  - Drop Handy's tray language submenu, update check and other distribution leftovers.
- **History window**: one place for all records, read from the JSONL journals. Spokenly's History is the reference: filter chips (All, Dictation, Files, Meetings, one chip per mode), search, newest/oldest sort.
  - Dictations: final text with the raw ASR text under it (dimmed), target app, age, recording duration; copy either text.
  - An audio-only meeting not yet transcribed shows as such, with a "Transcribe" action.
  - Meetings: list with title, date, duration; open one to read the transcript (live-updating while it records), copy or export as Markdown, open audio.
  - Search across both.
- **Settings window**: modes, engines, LLM keys, microphone, MCP toggle. Reuse Handy's settings components where they fit, delete the rest.
- **Recording overlay**: Handy's streaming overlay while dictating: live text as you speak, timer, cancel button; plus the active mode's name.

UI patterns taken from the upstream apps (seen in code and screenshots; still to be tried in the running apps):

- Handy, shortcut behavior **Auto** (hold to record, or tap to toggle) next to Hold and Toggle; per mode, together with double-tap.
- Handy, history entry actions: copy, star, **re-transcribe** (run again with another engine or mode; this is how Whisper and ElevenLabs get compared on the same audio), delete, inline audio player, "Open recordings folder" (opens `~/Hutarka`).
- Handy, model picker with per-language quality grades, speed and size. Its catalog also has non-Whisper models (Qwen3-ASR, Cohere Transcribe, ...) worth testing for Russian and Belarusian before reaching for ElevenLabs.
- Yogurt, tray while recording: "Recording · title", **Pause / Resume**, Finish; "Recent" submenu with the last 5 meetings.
- Yogurt, meeting page: transcript that follows the latest line while live, find in transcript, Copy menu, **Resume recording** (continue the same meeting as a new part), audio player.
- Yogurt, recording warnings surfaced to the user: no system audio, mic gone, device changed.
- Yogurt's floating recording widget (always on top, over full-screen apps) is **not** taken by default: it is visible when sharing the whole screen. The menu bar icon is enough.

## Meetings

Two recording modes, chosen when a recording starts (default set in Settings):

- **Audio only**: records mic and system audio to files (`me.m4a`, `them.m4a`), no ASR runs during the call. Near-zero CPU/GPU load. After the call: transcribe on demand from History or the menu bar, or automatically when the meeting ends (a setting).
- **Live transcript**: same audio files, plus ASR while the call runs, journal lines appear live and are readable over MCP.
- **Switching mid-call** from audio only to live is allowed: live transcription starts from now, and the part already recorded is transcribed from the file in the background (Yogurt calls this backfill).
- Either way the audio file is the source of truth; the after-meeting "better pass" (full transcription plus LLM correction) runs from the files.
- `yogurt-audio` already writes the files and streams PCM at the same time; in audio-only mode the PCM stream is simply not consumed.
- MCP for an audio-only meeting reports it as recording / not transcribed instead of returning empty lines.

## Storage

Plain files, no database, in an **open folder `~/Hutarka`** so they are easy to browse, grep and open in an editor:

```
~/Hutarka/
  meetings/2026-11-21_1430_<title>/   one folder per meeting
    me.m4a, them.m4a                  audio (Yogurt's format)
    live.jsonl                        lines written during the call (live mode)
    final.jsonl                       after-meeting pass: full transcription + LLM correction
  dictations/2026-11-21.jsonl         one file per day: dictations and translations
  lookups/2026-11-21.jsonl            one file per day: selection translator results
  files/<date>_<name>/transcript.jsonl  results of "Transcribe file"
```

A meeting's audio stays next to its transcripts (one folder = one meeting, easy to move or delete). The layout is a starting point; change it here first if it changes.

- **Transcripts are append-only JSONL journals**. One line = one transcript segment (`start_ms`, `end_ms`, `track`, `speaker`, `text`, `kind`: speech | dictation). The recorder appends; nothing rewrites a journal in place (a later "better pass" writes a separate file).
- **Live MCP reads the journal directly**: `get_lines_since(meeting, cursor)` returns lines after a cursor (line number or byte offset) and the new cursor. Same code path for live and finished meetings.
- **Search**: scan the files (personal scale, ripgrep-style); add an index only if it gets slow.
- **Dictation during a meeting**: the mic track picks it up; Hutarka knows the hotkey intervals, so those lines get `kind: dictation` in the meeting journal.
- **Settings**: Tauri store (JSON file) in the app data directory, as in Handy. Handy's SQLite history (`managers/history.rs`, `rusqlite`) is replaced by the JSONL journal.

## Signing and macOS permissions

No paid Apple Developer account. Locally built apps carry no quarantine attribute, so Gatekeeper and notarization do not apply.

TCC permissions (Microphone, Accessibility, Input Monitoring, System Audio Recording) are bound to the code signature. Ad-hoc signing (`signingIdentity: "-"`, Handy's default) changes identity on every rebuild and silently breaks granted permissions. Sign with a **stable local identity** instead: a self-signed code-signing certificate from Keychain Access, or a free Apple Development certificate from a personal Apple ID team. Never switch back to ad-hoc.

`Info.plist` must contain both `NSMicrophoneUsageDescription` and `NSAudioCaptureUsageDescription` (the second one is required for the Core Audio process tap prompt).

If a permission still looks enabled but does not work after a rebuild: `tccutil reset <Service> <bundle id>`, relaunch, grant again.

## Upstream sources

Local clones for reading and copying code (both MIT):

- `/opt/projects/Handy` (Handy v0.10.1). Dictation: hotkeys (`shortcut/`, `secure_input.rs`, `transcription_coordinator.rs`), paste (`clipboard.rs`, `paste_tx/`), isolated ASR worker (`engine_supervisor/`), LLM post-processing (`llm_client.rs`), pre-paste hook (`transcription_hook.rs`), settings (`settings.rs`).
- `/opt/projects/yogurt` (Yogurt v0.3.54, Electron/TS). Meetings: `app/native/yogurt-audio/main.swift` (mic + process tap, JSON-lines + PCM protocol on stdout, documented at the top of the file), live pipeline `app/src/main/asr/pipeline.ts` (per-track queues, echo removal, hallucination filter), MCP `app/src/main/mcp/`, MCP stdio relay `app/native/yogurt-mcp/main.swift`.

When code is copied from either project, keep its MIT copyright notice in `THIRD_PARTY_NOTICES.md`.

Research notes and step plans live in `docs_/` (git-ignored, the author's scratch space).

## Open questions

Tracked in `README.md` ("Open questions"). The latency question (dictation must not wait behind meeting transcription) is answered by measurement on a real call, not by guessing: two ASR workers vs. one worker with dictation priority.

## Conventions

- Rust: `cargo fmt` and `cargo clippy` clean; explicit error handling, no `unwrap` outside tests. One exception: `expect` on parsing a file embedded with `include_str!` (such as `resources/default-settings.json`) when a test parses the same file; a broken embedded file is a build bug, not a runtime error.
- Prefer deleting unused Handy code over feature-flagging it.
- Commit messages in English.
