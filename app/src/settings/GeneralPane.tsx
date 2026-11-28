import { useEffect, useState } from "react";
import { settings, type SettingsView } from "@/lib/api";
import { ApiKeySection } from "./ApiKeySection";

export function GeneralPane({ view, onChange }: { view: SettingsView; onChange: () => void }) {
  const [hotkey, setHotkey] = useState(view.hotkey);
  const [error, setError] = useState("");

  useEffect(() => setHotkey(view.hotkey), [view.hotkey]);

  // Like the API key's Save: only when there is something to save. The same
  // hotkey can be saved again while it is not active, to register it.
  const canSave = hotkey.trim() !== "" && (hotkey.trim() !== view.hotkey || !view.hotkey_active);

  const saveHotkey = async () => {
    if (!canSave) return;
    setError("");
    try {
      await settings.setHotkey(hotkey);
      onChange();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <>
      <h1 className="m-0 text-xl font-semibold">General</h1>
      <section className="flex flex-col gap-1.5">
        <label htmlFor="hotkey" className="font-medium">
          Translator hotkey
        </label>
        <div className="flex items-center gap-2">
          <input
            id="hotkey"
            value={hotkey}
            onChange={(e) => {
              setError("");
              setHotkey(e.target.value);
            }}
            onKeyDown={(e) => e.key === "Enter" && saveHotkey()}
            className="field h-7 w-56 rounded-md px-2"
            style={{ userSelect: "text" }}
          />
          <button
            type="button"
            onClick={saveHotkey}
            disabled={!canSave}
            className="btn-primary h-7 rounded-md px-3 text-[12px]"
          >
            Save
          </button>
        </div>
        {error ? (
          <span style={{ color: "var(--warn)" }}>{error}</span>
        ) : (
          !view.hotkey_active && (
            <span style={{ color: "var(--warn)" }}>
              {view.hotkey} is not active: another app may be using it. Free it and press Save, or
              choose another.
            </span>
          )
        )}
        <span className="text-[12px]" style={{ color: "var(--muted)" }}>
          A key combination such as Ctrl+Alt+T, Cmd+Shift+D or Alt+Space.
        </span>
      </section>
      <ApiKeySection view={view} onChange={onChange} />
    </>
  );
}
