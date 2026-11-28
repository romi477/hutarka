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

1. **API key.** Menu bar icon › Settings… › General: paste the Anthropic key and press Save. It is stored in the app's settings (`~/Library/Application Support/dev.hutarka.app/settings.json`), never in `~/Hutarka`.
2. **Accessibility.** The first lookup asks for it: System Settings › Privacy & Security › Accessibility, turn Hutarka on, then press the hotkey again. It is needed to read the selection and to paste.

## Translating a selection

Select text, press **⌃⌥T** (or menu bar › Translate selection).

**Typing instead of selecting.** Press the hotkey with nothing selected (or menu bar › Type text to translate…): the popup opens with an empty field. Type or paste, then ⌘↵ or Translate.

**The original** sits above the answer, collapsed to one line. Click it to open it, edit the text or write a new one, and translate again with ⌘↵ into the language shown in the header.

- **Target language.** Every lookup translates into the default language (English unless you change it in Settings › Translation). The model works out the language of the text itself, so any source language works. The button in the popup header (`→ EN`) opens the list of your languages; picking one, or ⌘1 … ⌘9, translates into it, for this lookup only. Hutarka does not check whether the text is already in the target language: pick another one when it is.
- **Word or text.** One to three words (no sentence punctuation) get a short dictionary entry: the translation on the first line, then meanings with part of speech, examples, collocations. Longer text gets a plain translation.
- **Explain** (checkbox in the header, ⌘E) gives a deeper answer instead. For a word or phrase: the translation, the meaning, five common sentences with translations, and a note on typical mistakes. For a sentence: its translation, then the idioms, phrasal verbs and grammar worth learning in it, with examples. The checkbox state is remembered.
- **Copy** (⌘C) copies the answer and closes the popup. If you selected part of the answer in the popup, ⌘C copies only that part and the popup stays open. Select with the mouse, or ⌘A for the whole answer. In the original field ⌘C, ⌘X, ⌘V, ⌘A and ⌘Z work as usual.
- **Replace** (↵) closes the popup and pastes the answer over the original selection, for editable fields. Whatever was on your clipboard (text, images, files) comes back afterwards. Not shown for typed text.
- **Move** the popup by dragging its header. It grows with the answer up to the screen's height; only a longer answer scrolls.
- **Esc**, the Close button at the bottom right, or a click outside closes the popup.

Popup keys:

| Key | Action |
|---|---|
| ⌘↵ | translate the text in the original field |
| ⌘C | copy the answer |
| ↵ | replace the selection with the answer (outside the original field; only for a selection) |
| ⌘E | toggle Explain and run again |
| ⌘1 … ⌘9 | run again into the first, second, … language of the list in Settings › Translation |
| Esc | close |

The header holds the target language, the Explain checkbox, the model with the time the answer took, and the close button.

## Settings

Menu bar › Settings… (⌘,).

- **General**: the hotkey (for example `Ctrl+Alt+T`, `Cmd+Shift+Y`) and the Anthropic API key.
- **Translation** (also menu bar › Translator prompts…): the languages you translate into and every prompt the translator sends.
  - **Translate into** lists your languages (shipped: English, Russian, Polish). **Add language…** adds one of English, Russian, Belarusian, Polish, German, Spanish, Greek, Italian, French, with a ready translation prompt. Each language has its own prompt, model and effort. **Make default** makes every lookup start with that language; **Delete** removes a language (not the default one). The order here is the order of the popup menu and of ⌘1 … ⌘9.
  - **Word lookup** and **Explain** are shared by all languages; `{to}` in them is the language chosen in the popup.
  - A prompt is saved when you leave the field; **Reset to default** brings back the shipped text.
- **Audio capture** and **Speech**: empty until meeting recording and dictation are built.

Edited prompts are kept as they are when a new version ships different default prompts. To get a new default, press Reset to default on that prompt. An edited Word lookup or Explain prompt from an older version may still contain `{from}`; it is filled with "the original language".

## Where things are saved

| What | Where |
|---|---|
| Lookups (one JSON line per finished answer: time, target language `to`, mode, text, result, model, ms) | `~/Hutarka/lookups/YYYY-MM-DD.jsonl` |
| Settings and API key | `~/Library/Application Support/dev.hutarka.app/settings.json` |
| Log (never contains the key or the selected text) | `~/Library/Logs/dev.hutarka.app/Hutarka.log` |

## Troubleshooting

- **"Accessibility permission needed" although Hutarka is on in System Settings.** The permission belongs to an older signature. Reset it, start the app, grant again:

  ```bash
  tccutil reset Accessibility dev.hutarka.app
  ```

- **"No Anthropic API key".** Settings › General, save the key.
- **The popup opens empty although text is selected (browser, Electron app).** These often hide the selection from Accessibility; Hutarka then copies it with ⌘C. Make sure the text is really selected and the window is focused, or paste the text into the field.
- **Something else fails.** Look at the log file above.

## Known limits

- The hotkey is a key combination. Single-key and double-tap triggers come later.
- Only Anthropic models.
