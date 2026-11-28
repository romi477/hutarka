import { useState } from "react";
import { settings, type SettingsView } from "@/lib/api";

/** The Anthropic API key: saved, replaced or removed; never shown. */
export function ApiKeySection({ view, onChange }: { view: SettingsView; onChange: () => void }) {
  const [key, setKey] = useState("");
  const [error, setError] = useState("");

  /** Shown as done only after the settings file is written. */
  const store = async (next: string) => {
    setError("");
    try {
      await settings.setApiKey(next);
      setKey("");
    } catch (e) {
      setError(String(e));
    }
    onChange();
  };

  const save = () => key.trim() && store(key);
  const remove = () => store("");

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
            className="field h-7 w-96 rounded-md px-2"
            style={{ userSelect: "text" }}
          />
          <button
            type="button"
            onClick={save}
            disabled={!key.trim()}
            className="btn-primary h-7 rounded-md px-3 text-[12px]"
          >
            Save
          </button>
          {view.has_api_key && (
            <button
              type="button"
              onClick={remove}
              className="btn h-7 rounded-md px-3 text-[12px]"
            >
              Remove
            </button>
          )}
        </div>
        {error && (
          <span className="text-[12px]" style={{ color: "var(--warn)" }}>
            {error}
          </span>
        )}
        <span className="text-[12px]" style={{ color: "var(--muted)" }}>
          {view.has_api_key ? "Key saved. " : "No key yet. "}
          Stored in Hutarka's settings in the app data folder, not in ~/Hutarka. The command-line
          tool reads HUTARKA_API_KEY instead.
        </span>
    </section>
  );
}
