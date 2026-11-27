# Hutarka

> *Гутарка* (Belarusian): a conversation, a talk.

**Status: early.** The `hutarka` command-line translator ([docs](docs/cli.md)) and the app's selection translator ([docs](docs/app.md)) work today; dictation and meetings are not built yet. This README is a sketch of what Hutarka is meant to become. Documentation of what already works is in [docs/](docs/README.md).

Hutarka is a **private, local-first voice assistant for the Mac** that combines two things that are usually two separate apps:

1. **Dictation anywhere.** Press a hotkey, speak, and the text (optionally translated or cleaned up by an LLM) is pasted into whatever app is focused.
2. **Meeting capture without a bot.** Record your microphone and the computer's system audio, transcribe the call live, and let AI tools read the transcript over MCP, during the meeting and after it.

One app, one audio pipeline, one transcript store.

## Why

Today this takes two tools running side by side: a dictation app for hotkey speech-to-text and translation, and a meeting recorder that captures system audio. They work, but they do not know about each other, their transcripts are not reachable from an AI agent while a meeting is still going on, and neither gives you one set of modes that covers both cases.

## Target scenarios

- **Dictate and translate.** Speak Russian (or Belarusian, Polish, ...) and get English text pasted at the cursor.
- **Dictate during a call.** You are screen-sharing in a meeting, and instead of typing into a field you hold the hotkey and speak. The meeting keeps recording, and the dictation is pasted with no noticeable delay.
- **Live meeting transcript for an agent.** Claude (or any MCP client) reads the transcript of the meeting that is happening right now and can answer questions or give hints.
- **Notes after the call.** Summary and searchable history of past meetings, all stored on your Mac.

## Planned features

**Dictation**
- Global hotkey, push-to-talk or toggle, paste into the active app.
- **Modes**: each mode has its own hotkey, language, speech model and LLM prompt (for example "clean up", "translate to English", "translate to Polish").
- LLM post-processing with your own keys (Anthropic, OpenAI, Ollama and other OpenAI-compatible endpoints) or fully local models.
- Hook script that can rewrite text right before it is pasted.

**Meetings**
- Microphone and system audio captured separately, no bot joins the call.
- Live transcript while the meeting runs, with a better pass saved afterwards.
- Dictation works while a meeting is being recorded. Dictated speech can be kept out of the meeting transcript or marked as dictation.
- Calendar-aware titles and attendees.

**MCP server (local, read-only to start with)**
- List meetings, get a transcript, search across meetings.
- **Live access**: find the meeting that is recording right now and read the lines added since a given point in time.
- Off by default and enabled explicitly in settings.

**Speech and language engines**
- Local: Whisper (whisper.cpp) and other on-device models; Apple on-device speech where available.
- Cloud, bring your own key: for example ElevenLabs Scribe or Deepgram, when you want streaming or higher accuracy.
- Local-only mode that blocks all network access.

## Platform

macOS on Apple Silicon only, at first (system audio capture relies on Core Audio process taps, macOS 14.4 or later). Windows and Linux are explicit non-goals for now.

## Open questions

- **Base**: fork an existing Electron-based app and strip it down, or build a native Swift app and reuse only the small native helpers.
- **Latency**: dictation must not wait behind meeting transcription. This needs a priority queue or a separate model instance, and has to be measured on a real call.
- **Live MCP**: polling (`get lines since ...`) first; push notifications later if clients support them.
- **Dictation vs. meeting transcript**: keep dictated speech out of the transcript, mark it, or make it a setting.

## Rough roadmap

1. Skeleton app: hotkey dictation with local Whisper, paste into the active app.
2. Modes with LLM post-processing and translation.
3. Meeting capture: microphone plus system audio, saved locally.
4. Live transcript while recording, dictation concurrent with recording.
5. Local MCP server with live access.
6. Cloud engines with your own keys, summaries, history search.

## Inspiration and credits

Hutarka is an idea built on the shoulders of two open-source projects, both MIT-licensed:

- [Yogurt](https://github.com/FrigadeHQ/yogurt): private meeting notes with native audio capture and a local MCP server.
- [Handy](https://github.com/cjpais/Handy): offline push-to-talk dictation with LLM post-processing and a transcription hook.

If any of their code is reused, their license notices will be kept in this repository.

## License

To be decided. (The projects above are MIT.)
