import { useEffect, useState } from "react";
import { settings, type SettingsView } from "@/lib/api";
import { GeneralPane } from "./GeneralPane";
import { LlmPane } from "./LlmPane";
import { TranslatorPane } from "./TranslatorPane";

const SECTIONS = [
  { id: "general", label: "General" },
  { id: "translator", label: "Translator" },
  { id: "llm", label: "LLM" },
] as const;

type Section = (typeof SECTIONS)[number]["id"];

function sectionFrom(value: string): Section {
  return SECTIONS.find((s) => s.id === value)?.id ?? "translator";
}

export function Settings() {
  const [section, setSection] = useState<Section>(sectionFrom(location.hash.slice(1)));
  const [view, setView] = useState<SettingsView | null>(null);

  const reload = () => settings.get().then(setView);

  useEffect(() => {
    reload();
    const off = settings.onSection((s) => {
      setSection(sectionFrom(s));
      reload();
    });
    // The window is reused: refresh when it comes back into view.
    const onFocus = () => reload();
    window.addEventListener("focus", onFocus);
    return () => {
      off.then((f) => f());
      window.removeEventListener("focus", onFocus);
    };
  }, []);

  return (
    <div className="flex h-screen" style={{ background: "var(--bg-solid)" }}>
      <nav
        aria-label="Settings sections"
        className="flex w-[200px] shrink-0 flex-col gap-0.5 border-r px-2.5 py-3"
        style={{ background: "var(--sidebar)", borderColor: "var(--line)" }}
      >
        {SECTIONS.map((s) => (
          <button
            key={s.id}
            type="button"
            aria-current={section === s.id ? "page" : undefined}
            onClick={() => setSection(s.id)}
            className="rounded-md px-2.5 py-1.5 text-left"
            style={section === s.id ? { background: "var(--chip-strong)", fontWeight: 500 } : undefined}
          >
            {s.label}
          </button>
        ))}
      </nav>
      <main className="flex min-w-0 flex-1 flex-col gap-4 px-7 py-5">
        {view &&
          (section === "general" ? (
            <GeneralPane view={view} onChange={reload} />
          ) : section === "llm" ? (
            <LlmPane view={view} onChange={reload} />
          ) : (
            <TranslatorPane view={view} onChange={reload} />
          ))}
      </main>
    </div>
  );
}
