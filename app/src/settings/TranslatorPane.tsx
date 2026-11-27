import { useEffect, useState } from "react";
import {
  DIRECTIONS,
  MODELS,
  settings,
  type Effort,
  type PromptId,
  type PromptSpec,
  type SettingsView,
} from "@/lib/api";

const ITEMS: { group: string; id: PromptId; label: string }[] = [
  ...DIRECTIONS.map((d) => ({ group: "Translate", id: `translate:${d.id}` as PromptId, label: d.label })),
  { group: "Any direction", id: "word", label: "Word lookup" },
  { group: "Any direction", id: "explain", label: "Explain" },
];

function specOf(view: SettingsView, id: PromptId): PromptSpec {
  if (id === "word") return view.prompts.word;
  if (id === "explain") return view.prompts.explain;
  return view.prompts.translate[id.slice("translate:".length) as keyof SettingsView["prompts"]["translate"]];
}

export function TranslatorPane({ view, onChange }: { view: SettingsView; onChange: () => void }) {
  const [selected, setSelected] = useState<PromptId>("explain");
  const [spec, setSpec] = useState<PromptSpec>(specOf(view, "explain"));
  const [status, setStatus] = useState("");

  // Load the selected prompt; keep unsaved edits of the open one on reloads.
  useEffect(() => setSpec(specOf(view, selected)), [selected]);

  const save = async (next: PromptSpec) => {
    setSpec(next);
    await settings.setPrompt(selected, next);
    setStatus("Saved");
    // The parent re-reads the settings, so switching prompts shows saved text.
    onChange();
  };

  const reset = async () => {
    const def = await settings.resetPrompt(selected);
    setSpec(def);
    setStatus("Default restored");
    onChange();
  };

  const pick = (id: PromptId) => {
    setStatus("");
    setSelected(id);
  };

  const usesPlaceholders = selected === "word" || selected === "explain";

  return (
    <>
      <div className="flex items-center gap-3">
        <h1 className="m-0 text-xl font-semibold">Translator</h1>
        <span className="flex-1" />
        <span className="text-[12px]" style={{ color: "var(--muted)" }}>
          Hotkey
        </span>
        <span
          className="rounded border px-2 py-0.5 text-[12px]"
          style={{ background: "var(--field)", borderColor: "var(--border)" }}
        >
          {view.hotkey}
        </span>
      </div>
      <div className="flex min-h-0 flex-1 gap-4">
        <div role="listbox" aria-label="Prompts" className="flex w-[190px] shrink-0 flex-col gap-0.5">
          {ITEMS.map((item, i) => (
            <div key={item.id} className="flex flex-col">
              {(i === 0 || ITEMS[i - 1].group !== item.group) && (
                <span
                  className="px-2.5 pt-3 pb-1 text-[11px] font-semibold tracking-wide uppercase first:pt-0"
                  style={{ color: "var(--muted)" }}
                >
                  {item.group}
                </span>
              )}
              <button
                type="button"
                role="option"
                aria-selected={selected === item.id}
                onClick={() => pick(item.id)}
                className="rounded-md px-2.5 py-1.5 text-left"
                style={
                  selected === item.id
                    ? { background: "var(--accent)", color: "var(--accent-fg)", fontWeight: 500 }
                    : undefined
                }
              >
                {item.label}
              </button>
            </div>
          ))}
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-2.5">
          <div className="flex items-center gap-2.5">
            <label htmlFor="model" className="text-[12px]" style={{ color: "var(--muted)" }}>
              Model
            </label>
            <select
              id="model"
              value={spec.model}
              onChange={(e) => save({ ...spec, model: e.target.value })}
              className="h-[26px] rounded-md border px-1.5 text-[12px]"
              style={{ background: "var(--field)", borderColor: "var(--border)" }}
            >
              {[...new Set([spec.model, ...MODELS])].map((m) => (
                <option key={m}>{m}</option>
              ))}
            </select>
            <label htmlFor="effort" className="text-[12px]" style={{ color: "var(--muted)" }}>
              Effort
            </label>
            <select
              id="effort"
              value={spec.effort}
              onChange={(e) => save({ ...spec, effort: e.target.value as Effort })}
              className="h-[26px] rounded-md border px-1.5 text-[12px]"
              style={{ background: "var(--field)", borderColor: "var(--border)" }}
            >
              <option>low</option>
              <option>medium</option>
              <option>high</option>
            </select>
          </div>
          <label htmlFor="prompt" className="text-[12px]" style={{ color: "var(--muted)" }}>
            System prompt
          </label>
          <textarea
            id="prompt"
            value={spec.system}
            onChange={(e) => {
              setStatus("");
              setSpec({ ...spec, system: e.target.value });
            }}
            onBlur={() => save(spec)}
            className="min-h-0 flex-1 resize-none rounded-lg border px-3 py-2.5 font-mono text-[12px] leading-relaxed"
            style={{ background: "var(--field)", borderColor: "var(--border)", userSelect: "text" }}
          />
          <div className="flex items-center gap-2">
            <span className="text-[12px]" style={{ color: "var(--muted)" }}>
              {usesPlaceholders ? (
                <>
                  <code>{"{from}"}</code> and <code>{"{to}"}</code> are filled from the direction.
                </>
              ) : (
                "Saved when you leave the field."
              )}
            </span>
            <span className="text-[12px]" style={{ color: "var(--muted)" }}>
              {status}
            </span>
            <span className="flex-1" />
            <button
              type="button"
              onClick={reset}
              className="h-7 rounded-md border px-3 text-[12px]"
              style={{ background: "var(--button)", borderColor: "var(--button-border)" }}
            >
              Reset to default
            </button>
          </div>
        </div>
      </div>
    </>
  );
}
