import { useState } from "react";
import { settings, type SettingsView } from "@/lib/api";

/** The Anthropic API key: saved, replaced or removed; never shown. */
export function ApiKeySection({ view, onChange }: { view: SettingsView; onChange: () => void }) {
  const [key, setKey] = useState("");

  const save = async () => {
    if (!key.trim()) return;
    await settings.setApiKey(key);
    setKey("");
    onChange();
  };

  const remove = async () => {
    await settings.setApiKey("");
    onChange();
  };

  return (
    <section className="flex flex-col gap-1.5">
        <label htmlFor="api-key" className="font-medium">
          Anthropic API key
        </label>
        <div className="flex items-center gap-2">
          <input
            id="api-key"
            type="password"
            autoComplete="off"
            placeholder={view.has_api_key ? "Key saved — paste a new one to replace it" : "sk-ant-…"}
            value={key}
            onChange={(e) => setKey(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && save()}
            className="h-7 w-96 rounded-md border px-2"
            style={{ background: "var(--field)", borderColor: "var(--border)", userSelect: "text" }}
          />
          <button
            type="button"
            onClick={save}
            disabled={!key.trim()}
            className="h-7 rounded-md px-3 text-[12px] font-medium disabled:opacity-50"
            style={{ background: "var(--accent)", color: "var(--accent-fg)" }}
          >
            Save
          </button>
          {view.has_api_key && (
            <button
              type="button"
              onClick={remove}
              className="h-7 rounded-md border px-3 text-[12px]"
              style={{ background: "var(--button)", borderColor: "var(--button-border)" }}
            >
              Remove
            </button>
          )}
        </div>
        <span className="text-[12px]" style={{ color: "var(--muted)" }}>
          {view.has_api_key ? "Key saved. " : "No key yet. "}
          Stored in Hutarka's settings in the app data folder, not in ~/Hutarka. The command-line
          tool reads HUTARKA_API_KEY instead.
        </span>
    </section>
  );
}
