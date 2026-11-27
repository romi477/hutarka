import { DIRECTIONS, type Direction } from "@/lib/api";

interface Props {
  current: Direction;
  onPick: (d: Direction) => void;
  onEditPrompts: () => void;
}

/** The direction button's menu: four directions (⌘1–⌘4) and "Edit prompts…". */
export function DirectionMenu({ current, onPick, onEditPrompts }: Props) {
  return (
    <div
      role="menu"
      aria-label="Direction"
      className="absolute left-0 top-7 z-10 flex w-[210px] flex-col gap-px rounded-lg border p-[5px] shadow-xl"
      style={{ background: "var(--bg-solid)", borderColor: "var(--border)" }}
    >
      {DIRECTIONS.map((d, i) => {
        const on = d.id === current;
        return (
          <button
            key={d.id}
            type="button"
            role="menuitemradio"
            aria-checked={on}
            onClick={() => onPick(d.id)}
            className="flex h-6 items-center gap-2 rounded px-2 text-left text-[13px] hover:bg-[var(--accent)] hover:text-white"
          >
            <span className="w-3">{on ? "✓" : ""}</span>
            <span className="flex-1">{d.label}</span>
            <span className="text-[11px] opacity-60">⌘{i + 1}</span>
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
