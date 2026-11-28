import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Kind = "text" | "word" | "explain";
export type Status = "idle" | "streaming" | "done" | "error";
export type ErrorKind = "no_key" | "not_trusted" | "too_long" | "api";
export type Lang = "en" | "ru" | "be" | "pl" | "de" | "es" | "el" | "it" | "fr";
export type Effort = "low" | "medium" | "high";

export const LANG_NAMES: Record<Lang, string> = {
  en: "English",
  ru: "Russian",
  be: "Belarusian",
  pl: "Polish",
  de: "German",
  es: "Spanish",
  el: "Greek",
  it: "Italian",
  fr: "French",
};

export interface Snapshot {
  id: number;
  status: Status;
  text: string;
  /** The language this lookup translates into. */
  to: Lang;
  /** Every target card in Settings order: the menu and ⌘1…⌘9. */
  targets: Lang[];
  kind: Kind;
  explain: boolean;
  result: string;
  model: string;
  ms: number;
  error_kind: ErrorKind | null;
  error: string;
  from_selection: boolean;
  /** Set when the answer could not be written to the history journal. */
  journal_error: string;
}

export const popup = {
  snapshot: () => invoke<Snapshot | null>("popup_snapshot"),
  onSnapshot: (cb: (s: Snapshot) => void): Promise<UnlistenFn> =>
    listen<Snapshot>("popup:snapshot", (e) => cb(e.payload)),
  rerun: (to: Lang, explain: boolean) => invoke("rerun", { to, explain }),
  choose: (to: Lang, explain: boolean) => invoke("choose_target", { to, explain }),
  translate: (text: string, to: Lang, explain: boolean, fromSelection: boolean) =>
    invoke("translate_text", { text, to, explain, fromSelection }),
  copy: () => invoke("copy_result"),
  clear: () => invoke("clear_popup"),
  pin: (pinned: boolean) => invoke("pin_popup", { pinned }),
  pinned: () => invoke<boolean>("popup_pinned"),
  onPinned: (cb: (pinned: boolean) => void): Promise<UnlistenFn> =>
    listen<boolean>("popup:pinned", (e) => cb(e.payload)),
  copyText: (text: string) => invoke("copy_text", { text }),
  clipboardText: () => invoke<string>("clipboard_text"),
  replace: () => invoke("replace_result"),
  close: () => invoke("close_popup"),
  resize: (height: number, width: number) => invoke("resize_popup", { height, width }),
  dragged: () => invoke("popup_dragged"),
  openSettings: (section?: string) => invoke("open_settings", { section: section ?? null }),
  openAccessibility: () => invoke("open_accessibility"),
};

export type PromptId = `translate:${Lang}` | "word" | "explain";

export interface PromptSpec {
  system: string;
  model: string;
  effort: Effort;
}

export interface Target {
  lang: Lang;
  default: boolean;
  prompt: PromptSpec;
}

export interface SettingsView {
  has_api_key: boolean;
  /** Last six characters of the saved key. */
  api_key_end: string | null;
  hotkey: string;
  /** False when the hotkey could not be registered (taken by another app). */
  hotkey_active: boolean;
  explain: boolean;
  targets: Target[];
  prompts: {
    word: PromptSpec;
    explain: PromptSpec;
  };
  /** Every language a card can have, in list order. */
  languages: Lang[];
}

export const MODELS = ["claude-haiku-5-5", "claude-sonnet-5-5"];

export const settings = {
  get: () => invoke<SettingsView>("get_settings"),
  setPrompt: (id: PromptId, spec: PromptSpec) => invoke("set_prompt", { id, spec }),
  resetPrompt: (id: PromptId) => invoke<PromptSpec>("reset_prompt", { id }),
  setApiKey: (key: string) => invoke("set_api_key", { key }),
  addTarget: (lang: Lang) => invoke("add_target", { lang }),
  removeTarget: (lang: Lang) => invoke("remove_target", { lang }),
  setDefaultTarget: (lang: Lang) => invoke("set_default_target", { lang }),
  setHotkey: (hotkey: string) => invoke("set_hotkey", { hotkey }),
  onSection: (cb: (section: string) => void): Promise<UnlistenFn> =>
    listen<string>("settings:section", (e) => cb(e.payload)),
};
