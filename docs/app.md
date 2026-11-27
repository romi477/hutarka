# Hutarka app

A menu bar app. Today it does one thing: the **selection translator**. Select text in any app, press the hotkey, and a small popup near the mouse shows the translation, streamed as it arrives.

Dictation modes and meeting capture are not built yet.

## Install

Needs Rust, Node.js (`brew install node`) and an Anthropic API key.

Once per Mac, create the local code-signing certificate. macOS ties the Accessibility permission to the app's signature; with this stable identity the permission survives rebuilds:

```bash
scripts/make-signing-cert.sh
```

Build, install to `~/Applications/Hutarka.app` and start it (a running copy is quit first):

```bash
scripts/app.sh install
```

Other commands of the same script:

| Command | What it does |
|---|---|
| `scripts/app.sh build` | build the signed `target/release/bundle/macos/Hutarka.app` only |
| `scripts/app.sh install` | build, replace `~/Applications/Hutarka.app`, start it |
| `scripts/app.sh cli` | install the `hutarka` CLI into `~/.cargo/bin` |
| `scripts/app.sh all` | `install` and `cli` |

`APP_DEST=/path/Hutarka.app` installs elsewhere; `NO_LAUNCH=1` does not start the app after installing.

The app has no Dock icon. It lives in the menu bar.

## First run

1. **API key.** Menu bar icon › Settings… › LLM: paste the Anthropic key and press Save. It is stored in the app's settings (`~/Library/Application Support/dev.hutarka.app/settings.json`), never in `~/Hutarka`.
2. **Accessibility.** The first lookup asks for it: System Settings › Privacy & Security › Accessibility, turn Hutarka on, then press the hotkey again. It is needed to read the selection and to paste.

## Translating a selection

Select text, press **⌃⌥T** (or menu bar › Translate selection).

- **Direction** is automatic: English goes to Russian, Polish and Russian go to English. The direction button in the popup header switches between EN → RU, EN → PL, PL → EN and RU → EN. Choosing EN → RU or EN → PL by hand makes it the default for English from then on.
- **Word or text.** One to three words (no sentence punctuation) get a short dictionary entry: meanings with part of speech, examples, collocations. Longer text gets a plain translation.
- **Explain** (checkbox, ⌘E) gives a deeper answer instead: the meaning, five common sentences with translations, and a note on typical mistakes. The checkbox state is remembered.
- **Copy** (⌘C) copies the answer. If you selected part of the answer in the popup, ⌘C copies only that part.
- **Replace** (↵) pastes the answer over the original selection, for editable fields.
- **Move** the popup by dragging its header. It grows with the answer up to the screen's height; only a longer answer scrolls.
- **Esc** or a click outside closes the popup.

Popup keys:

| Key | Action |
|---|---|
| ⌘C | copy the answer |
| ↵ | replace the selection with the answer |
| ⌘E | toggle Explain and run again |
| ⌘1 … ⌘4 | run again as EN → RU, EN → PL, PL → EN, RU → EN |
| Esc | close |

The header shows the model and the time the answer took.

## Settings

Menu bar › Settings… (⌘,).

- **General**: the hotkey (for example `Ctrl+Alt+T`, `Cmd+Shift+Y`), and where English text goes by default (Russian or Polish).
- **Translator** (also menu bar › Translator prompts…): every prompt the translator sends, editable. One translation prompt per direction, plus Word lookup and Explain, which use `{from}` and `{to}` placeholders filled from the direction. Each prompt has its own model and effort. A prompt is saved when you leave the field; **Reset to default** brings back the shipped text.
- **LLM**: the Anthropic API key.

Edited prompts are kept as they are when a new version ships different default prompts. To get a new default, press Reset to default on that prompt.

## Where things are saved

| What | Where |
|---|---|
| Lookups (one JSON line per finished answer: time, direction, mode, text, result, model, ms) | `~/Hutarka/lookups/YYYY-MM-DD.jsonl` |
| Settings and API key | `~/Library/Application Support/dev.hutarka.app/settings.json` |
| Log (never contains the key or the selected text) | `~/Library/Logs/dev.hutarka.app/Hutarka.log` |

## Troubleshooting

- **"Accessibility permission needed" although Hutarka is on in System Settings.** The permission belongs to an older signature. Reset it, start the app, grant again:

  ```bash
  tccutil reset Accessibility dev.hutarka.app
  ```

- **"No Anthropic API key".** Settings › LLM, save the key.
- **"Nothing selected" in a browser or Electron app.** These often hide the selection from Accessibility; Hutarka then copies it with ⌘C. Make sure the text is really selected and the window is focused.
- **Something else fails.** Look at the log file above.

## Known limits

- Replace and the ⌘C fallback restore only text on the clipboard. Images or files copied before are lost.
- The hotkey is a key combination. Single-key and double-tap triggers come later.
- Only Anthropic models.
