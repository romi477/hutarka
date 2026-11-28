import { LANG_NAMES, type Lang } from "@/lib/api";

interface Props {
  current: Lang;
  targets: Lang[];
  onPick: (to: Lang) => void;
  onEditPrompts: () => void;
}

/** The target button's menu: the cards from Settings (⌘1–⌘9) and "Edit prompts…". */
export function TargetMenu({ current, targets, onPick, onEditPrompts }: Props) {
  return (
    <div
      role="menu"
      aria-label="Translate into"
      className="absolute left-0 top-7 z-10 flex w-[210px] flex-col gap-px rounded-lg border p-[5px] shadow-xl"
      style={{ background: "var(--bg-solid)", borderColor: "var(--border)" }}
    >
      {targets.map((lang, i) => {
        const on = lang === current;
        return (
          <button
            key={lang}
            type="button"
            role="menuitemradio"
            aria-checked={on}
            onClick={() => onPick(lang)}
            className="flex h-6 items-center gap-2 rounded px-2 text-left text-[13px] hover:bg-[var(--accent)] hover:text-white"
          >
            <span className="w-3">{on ? "✓" : ""}</span>
            <span className="flex-1">→ {LANG_NAMES[lang]}</span>
            {i < 9 && <span className="text-[11px] opacity-60">⌘{i + 1}</span>}
          </button>
        );
      })}
      <div className="mx-2 my-1 h-px" style={{ background: "var(--line)" }} />
      <button
        type="button"
        role="menuitem"
        onClick={onEditPrompts}
        className="flex h-6 items-center gap-2 rounded px-2 text-left text-[13px] hover:bg-[var(--accent)] hover:text-white"
      >
        <span className="w-3" />
        <span className="flex-1">Edit prompts…</span>
      </button>
    </div>
  );
}
