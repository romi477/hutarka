import { useEffect, useState } from "react";
import { settings, type Lang, type SettingsView } from "@/lib/api";

export function GeneralPane({ view, onChange }: { view: SettingsView; onChange: () => void }) {
  const [hotkey, setHotkey] = useState(view.hotkey);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => setHotkey(view.hotkey), [view.hotkey]);

  const saveHotkey = async () => {
    setError("");
    setSaved(false);
    try {
      await settings.setHotkey(hotkey);
      setSaved(true);
      onChange();
    } catch (e) {
      setError(String(e));
    }
  };

  const setTarget = async (lang: Lang) => {
    await settings.setEnglishTarget(lang);
    onChange();
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
            onChange={(e) => setHotkey(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && saveHotkey()}
            className="h-7 w-56 rounded-md border px-2"
            style={{ background: "var(--field)", borderColor: "var(--border)", userSelect: "text" }}
          />
          <button
            type="button"
            onClick={saveHotkey}
            className="h-7 rounded-md border px-3 text-[12px]"
            style={{ background: "var(--button)", borderColor: "var(--button-border)" }}
          >
            Save
          </button>
          {saved && <span style={{ color: "var(--muted)" }}>Saved</span>}
        </div>
        {error && <span style={{ color: "var(--warn)" }}>{error}</span>}
        <span className="text-[12px]" style={{ color: "var(--muted)" }}>
          A key combination such as Ctrl+Alt+T, Cmd+Shift+D or Alt+Space.
        </span>
      </section>
      <section className="flex flex-col gap-1.5">
        <span className="font-medium">English text is translated to</span>
        <div className="flex gap-4">
          {(["ru", "pl"] as const).map((lang) => (
            <label key={lang} className="flex items-center gap-1.5">
              <input
                type="radio"
                name="english-target"
                checked={view.english_target === lang}
                onChange={() => setTarget(lang)}
                style={{ accentColor: "var(--accent)" }}
              />
              {lang === "ru" ? "Russian" : "Polish"}
            </label>
          ))}
        </div>
        <span className="text-[12px]" style={{ color: "var(--muted)" }}>
          Picking a direction in the popup changes this too.
        </span>
      </section>
    </>
  );
}
