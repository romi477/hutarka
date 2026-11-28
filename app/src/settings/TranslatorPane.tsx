import { useEffect, useState } from "react";
import {
  LANG_NAMES,
  MODELS,
  settings,
  type Effort,
  type Lang,
  type PromptId,
  type PromptSpec,
  type SettingsView,
} from "@/lib/api";

interface Item {
  group: string;
  id: PromptId;
  label: string;
  /** Set for a target-language card. */
  lang?: Lang;
  isDefault?: boolean;
}

function itemsOf(view: SettingsView): Item[] {
  return [
    ...view.targets.map((t) => ({
      group: "Translate into",
      id: `translate:${t.lang}` as PromptId,
      label: LANG_NAMES[t.lang],
      lang: t.lang,
      isDefault: t.default,
    })),
    { group: "Any language", id: "word", label: "Word lookup" },
    { group: "Any language", id: "explain", label: "Explain" },
  ];
}

/** The prompt of `id`; undefined for a card not in `view` (yet, or any more). */
function specOf(view: SettingsView, id: PromptId): PromptSpec | undefined {
  if (id === "word") return view.prompts.word;
  if (id === "explain") return view.prompts.explain;
  const lang = id.slice("translate:".length);
  return view.targets.find((t) => t.lang === lang)?.prompt;
}

export function TranslatorPane({
  view,
  onChange,
}: {
  view: SettingsView;
  /** Re-reads the settings; resolves once `view` is fresh. */
  onChange: () => Promise<unknown>;
}) {
  const [selected, setSelected] = useState<PromptId>("explain");
  const [spec, setSpec] = useState<PromptSpec>(view.prompts.explain);
  const [status, setStatus] = useState("");

  const items = itemsOf(view);
  const current = items.find((i) => i.id === selected);
  const unused = view.languages.filter((l) => !view.targets.some((t) => t.lang === l));

  // Load the selected prompt; keep unsaved edits of the open one on reloads.
  useEffect(() => {
    const next = specOf(view, selected);
    if (next) setSpec(next);
  }, [selected]);

  // The selected card was removed: fall back to Explain.
  useEffect(() => {
    if (!current) setSelected("explain");
  }, [current]);

  /** Runs a card action; a refusal from the backend shows as the status. */
  const act = async (action: () => Promise<unknown>, done: string) => {
    try {
      await action();
      setStatus(done);
    } catch (e) {
      setStatus(String(e));
    }
    await onChange();
  };

  const add = async (lang: Lang) => {
    await act(() => settings.addTarget(lang), `${LANG_NAMES[lang]} added`);
    // After the reload, so the new card is in `view` when it is selected.
    setSelected(`translate:${lang}`);
  };

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


  return (
    <>
      <div className="flex items-center gap-3">
        <h1 className="m-0 text-xl font-semibold">Translation</h1>
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
          {items.map((item, i) => (
            <div key={item.id} className="flex flex-col">
              {(i === 0 || items[i - 1].group !== item.group) && (
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
                {item.isDefault && <span className="opacity-60"> · default</span>}
              </button>
              {item.lang && items[i + 1]?.group !== item.group && unused.length > 0 && (
                <select
                  aria-label="Add language"
                  value=""
                  onChange={(e) => e.target.value && add(e.target.value as Lang)}
                  className="mt-1 h-[26px] rounded-md border px-1.5 text-[12px]"
                  style={{ background: "var(--field)", borderColor: "var(--border)" }}
                >
                  <option value="">Add language…</option>
                  {unused.map((l) => (
                    <option key={l} value={l}>
                      {LANG_NAMES[l]}
                    </option>
                  ))}
                </select>
              )}
            </div>
          ))}
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-2.5">
          {current?.lang && (
            <div className="flex items-center gap-2">
              <span className="min-w-0 truncate font-semibold">Translate into {current.label}</span>
              <span className="flex-1" />
              {current.isDefault ? (
                <span className="text-[12px]" style={{ color: "var(--muted)" }}>
                  Default: every lookup starts here
                </span>
              ) : (
                <>
                  <button
                    type="button"
                    onClick={() =>
                      act(() => settings.setDefaultTarget(current.lang!), "Every lookup now starts here")
                    }
                    className="h-[26px] shrink-0 whitespace-nowrap rounded-md border px-3 text-[12px]"
                    style={{ background: "var(--button)", borderColor: "var(--button-border)" }}
                  >
                    Make default
                  </button>
                  <button
                    type="button"
                    onClick={() => act(() => settings.removeTarget(current.lang!), "Removed")}
                    className="h-[26px] shrink-0 whitespace-nowrap rounded-md border px-3 text-[12px]"
                    style={{ background: "var(--button)", borderColor: "var(--button-border)" }}
                  >
                    Delete
                  </button>
                </>
              )}
            </div>
          )}
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
              {current?.lang ? (
                "Saved when you leave the field."
              ) : (
                <>
                  <code>{"{to}"}</code> is the language chosen in the popup.
                </>
              )}
            </span>
            <span className="text-[12px]" style={{ color: "var(--muted)" }}>
              {status}
            </span>
            <span className="flex-1" />
            <button
              type="button"
              onClick={reset}
              className="h-7 shrink-0 whitespace-nowrap rounded-md border px-3 text-[12px]"
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
