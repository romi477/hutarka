import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Direction = "en-ru" | "en-pl" | "pl-en" | "ru-en";
export type Kind = "text" | "word" | "explain";
export type Status = "idle" | "streaming" | "done" | "error";
export type ErrorKind = "no_key" | "not_trusted" | "too_long" | "api";
export type Lang = "en" | "pl" | "ru";
export type Effort = "low" | "medium" | "high";

export const DIRECTIONS: { id: Direction; label: string; from: string; to: string }[] = [
  { id: "en-ru", label: "English → Russian", from: "EN", to: "RU" },
  { id: "en-pl", label: "English → Polish", from: "EN", to: "PL" },
  { id: "pl-en", label: "Polish → English", from: "PL", to: "EN" },
  { id: "ru-en", label: "Russian → English", from: "RU", to: "EN" },
];

export interface Snapshot {
  id: number;
  status: Status;
  text: string;
  direction: Direction;
  kind: Kind;
  explain: boolean;
  result: string;
  model: string;
  ms: number;
  error_kind: ErrorKind | null;
  error: string;
  from_selection: boolean;
}

export const popup = {
  snapshot: () => invoke<Snapshot | null>("popup_snapshot"),
  onSnapshot: (cb: (s: Snapshot) => void): Promise<UnlistenFn> =>
    listen<Snapshot>("popup:snapshot", (e) => cb(e.payload)),
  rerun: (direction: Direction, explain: boolean) => invoke("rerun", { direction, explain }),
  translate: (text: string, direction: Direction | null, explain: boolean, fromSelection: boolean) =>
    invoke("translate_text", { text, direction, explain, fromSelection }),
  copy: () => invoke("copy_result"),
  copyText: (text: string) => invoke("copy_text", { text }),
  clipboardText: () => invoke<string>("clipboard_text"),
  replace: () => invoke("replace_result"),
  close: () => invoke("close_popup"),
  resize: (height: number) => invoke("resize_popup", { height }),
  dragged: () => invoke("popup_dragged"),
  openSettings: (section?: string) => invoke("open_settings", { section: section ?? null }),
  openAccessibility: () => invoke("open_accessibility"),
};

export type PromptId = `translate:${Direction}` | "word" | "explain";

export interface PromptSpec {
  system: string;
  model: string;
  effort: Effort;
}

export interface SettingsView {
  has_api_key: boolean;
  hotkey: string;
  english_target: Lang;
  explain: boolean;
  prompts: {
    translate: Record<Direction, PromptSpec>;
    word: PromptSpec;
    explain: PromptSpec;
  };
}

export const MODELS = ["claude-haiku-5-5", "claude-sonnet-5-5"];

export const settings = {
  get: () => invoke<SettingsView>("get_settings"),
  setPrompt: (id: PromptId, spec: PromptSpec) => invoke("set_prompt", { id, spec }),
  resetPrompt: (id: PromptId) => invoke<PromptSpec>("reset_prompt", { id }),
  setApiKey: (key: string) => invoke("set_api_key", { key }),
  setEnglishTarget: (lang: Lang) => invoke("set_english_target", { lang }),
  setHotkey: (hotkey: string) => invoke("set_hotkey", { hotkey }),
  onSection: (cb: (section: string) => void): Promise<UnlistenFn> =>
    listen<string>("settings:section", (e) => cb(e.payload)),
};
