import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { DIRECTIONS, popup, type Direction, type Snapshot } from "@/lib/api";
import { DirectionMenu } from "./DirectionMenu";

/** Room around the card for its shadow; matches the window width (560 = 520 + 2 × 20). */
const MARGIN = 20;
const MENU_HEIGHT = 170;
/** Header, footer and margins around the answer, so the window fits the screen. */
const CHROME_HEIGHT = 140;

const ERROR_TITLES: Record<string, string> = {
  no_key: "No Anthropic API key",
  not_trusted: "Accessibility permission needed",
  nothing: "Nothing selected",
  too_long: "Selection too long",
  api: "The translation failed",
};

const ERROR_HINTS: Record<string, string> = {
  no_key: "Add your key in Settings › LLM to translate. It is stored in app settings, not in ~/Hutarka.",
  not_trusted:
    "Allow Hutarka in System Settings › Privacy & Security › Accessibility, then try again.",
  nothing: "Select some text in any app, then press the hotkey.",
};

function Arrow() {
  return (
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M2 6h8M7 3l3 3-3 3" />
    </svg>
  );
}

export function Popup() {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [menu, setMenu] = useState(false);
  const wrapper = useRef<HTMLDivElement>(null);

  useEffect(() => {
    popup.snapshot().then((s) => s && setSnap(s));
    const off = popup.onSnapshot((s) => {
      setSnap(s);
      setMenu(false);
    });
    return () => {
      off.then((f) => f());
    };
  }, []);

  // The window follows the card's height.
  useLayoutEffect(() => {
    const el = wrapper.current;
    if (!el) return;
    const send = () => popup.resize(Math.ceil(el.offsetHeight + (menu ? MENU_HEIGHT : 0)));
    send();
    const observer = new ResizeObserver(send);
    observer.observe(el);
    return () => observer.disconnect();
  }, [menu]);

  const rerun = useCallback(
    (direction: Direction, explain: boolean) => {
      setMenu(false);
      popup.rerun(direction, explain);
    },
    [],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!snap) return;
      if (e.key === "Escape") {
        e.preventDefault();
        if (menu) setMenu(false);
        else popup.close();
      } else if (e.metaKey && e.key.toLowerCase() === "c") {
        // Copy the whole answer unless the user selected part of it.
        if (!window.getSelection()?.toString()) {
          e.preventDefault();
          popup.copy();
        }
      } else if (e.key === "Enter" && snap.status === "done") {
        e.preventDefault();
        popup.replace();
      } else if (e.metaKey && e.key.toLowerCase() === "e" && snap.text) {
        e.preventDefault();
        rerun(snap.direction, !snap.explain);
      } else if (e.metaKey && ["1", "2", "3", "4"].includes(e.key) && snap.text) {
        e.preventDefault();
        rerun(DIRECTIONS[Number(e.key) - 1].id, snap.explain);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [snap, menu, rerun]);

  if (!snap) return null;

  const dir = DIRECTIONS.find((d) => d.id === snap.direction) ?? DIRECTIONS[0];
  const streaming = snap.status === "streaming";
  const done = snap.status === "done";
  const showsWord = snap.kind !== "text" && snap.text.trim().length <= 60;

  return (
    <div ref={wrapper} style={{ padding: MARGIN }}>
      <div
        className="relative flex flex-col rounded-xl border"
        style={{
          background: "var(--bg)",
          borderColor: "var(--border)",
          boxShadow: "0 12px 32px rgba(0,0,0,0.22), 0 2px 6px rgba(0,0,0,0.10)",
          backdropFilter: "blur(20px)",
        }}
      >
        {snap.status === "error" ? (
          <ErrorBody snap={snap} />
        ) : (
          <>
            {/* Drag the header to move the popup (only elements marked as drag regions start a drag). */}
            <div
              data-tauri-drag-region
              onMouseDown={(e) => {
                if ((e.target as HTMLElement).hasAttribute("data-tauri-drag-region")) popup.dragged();
              }}
              className="flex cursor-grab items-center gap-2 pt-2.5 pr-2.5 pl-3.5"
            >
              {showsWord && (
                <span data-tauri-drag-region className="truncate text-[17px] font-semibold" style={{ maxWidth: 300 }}>
                  {snap.text.trim()}
                </span>
              )}
              <div className="relative">
                <button
                  type="button"
                  aria-label="Change direction"
                  aria-expanded={menu}
                  onClick={() => setMenu(!menu)}
                  className="flex h-6 items-center gap-1 rounded-md px-2 text-[12px] font-medium"
                  style={{ background: menu ? "var(--chip-strong)" : "var(--chip)", color: "var(--fg-2)" }}
                >
                  {dir.from}
                  <Arrow />
                  {dir.to}
                  <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                    <path d="M2.5 4l2.5 2.5L7.5 4" />
                  </svg>
                </button>
                {menu && (
                  <DirectionMenu
                    current={snap.direction}
                    onPick={(d) => rerun(d, snap.explain)}
                    onEditPrompts={() => popup.openSettings("translator")}
                  />
                )}
              </div>
              <span data-tauri-drag-region className="flex-1 self-stretch" />
              <span data-tauri-drag-region className="flex items-center gap-1.5 text-[11px]" style={{ color: "var(--muted)" }}>
                {streaming ? (
                  <>
                    <svg className="animate-spin" width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" aria-hidden="true">
                      <path d="M6 1.5a4.5 4.5 0 1 1-4.5 4.5" />
                    </svg>
                    {snap.kind === "explain" ? "Explaining…" : "Translating…"}
                  </>
                ) : (
                  done && `${modelName(snap.model)} · ${(snap.ms / 1000).toFixed(1)} s`
                )}
              </span>
              <button
                type="button"
                aria-label="Close"
                onClick={() => popup.close()}
                className="flex h-6 w-6 items-center justify-center rounded-md"
                style={{ color: "var(--muted)" }}
              >
                <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" aria-hidden="true">
                  <path d="M3 3l6 6M9 3l-6 6" />
                </svg>
              </button>
            </div>
            <div
              className="overflow-y-auto px-4 pt-2 pb-3.5 text-[14px] leading-normal whitespace-pre-wrap"
              style={{ maxHeight: window.screen.availHeight - CHROME_HEIGHT, userSelect: "text", cursor: "text" }}
            >
              <Formatted text={snap.result} />
              {streaming && (
                <span
                  className="ml-0.5 inline-block h-4 w-0.5 animate-pulse align-[-3px]"
                  style={{ background: "var(--accent)" }}
                />
              )}
            </div>
            <div
              className="flex items-center gap-1.5 rounded-b-xl border-t px-2.5 py-2"
              style={{ borderColor: "var(--line)", background: "var(--footer)" }}
            >
              <button
                type="button"
                disabled={!done}
                onClick={() => popup.copy()}
                className="flex h-7 items-center gap-1.5 rounded-md border px-2.5 text-[12px] disabled:opacity-50"
                style={{ background: "var(--button)", borderColor: "var(--button-border)" }}
              >
                Copy <span className="text-[11px]" style={{ color: "var(--muted)" }}>⌘C</span>
              </button>
              <button
                type="button"
                disabled={!done}
                onClick={() => popup.replace()}
                className="flex h-7 items-center gap-1.5 rounded-md px-2.5 text-[12px] font-medium disabled:opacity-50"
                style={{ background: "var(--accent)", color: "var(--accent-fg)" }}
              >
                Replace <span className="text-[11px] opacity-80">↵</span>
              </button>
              <span className="flex-1" />
              <label className="flex h-7 items-center gap-1.5 px-1.5 text-[12px]">
                <input
                  type="checkbox"
                  checked={snap.explain}
                  onChange={() => rerun(snap.direction, !snap.explain)}
                  className="m-0 h-3.5 w-3.5"
                  style={{ accentColor: "var(--accent)" }}
                />
                Explain <span className="text-[11px]" style={{ color: "var(--muted)" }}>⌘E</span>
              </label>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

function ErrorBody({ snap }: { snap: Snapshot }) {
  const kind = snap.error_kind ?? "api";
  const action =
    kind === "no_key"
      ? { label: "Open Settings", run: () => popup.openSettings("llm") }
      : kind === "not_trusted"
        ? { label: "Open Accessibility Settings", run: () => popup.openAccessibility() }
        : null;
  return (
    <>
      <div className="flex items-start gap-2.5 pt-3.5 pr-2.5 pb-3 pl-4">
        <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="var(--warn)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" role="img" aria-label="Warning" className="mt-px shrink-0">
          <path d="M9 2.5l7 12.5H2z" />
          <path d="M9 7.5v3.5M9 13.2v.1" />
        </svg>
        <div className="flex flex-1 flex-col gap-1">
          <span className="font-semibold">{ERROR_TITLES[kind]}</span>
          <span className="leading-snug" style={{ color: "var(--fg-2)", userSelect: "text" }}>
            {ERROR_HINTS[kind] ?? snap.error}
          </span>
        </div>
        <button
          type="button"
          aria-label="Close"
          onClick={() => popup.close()}
          className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md"
          style={{ color: "var(--muted)" }}
        >
          <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" aria-hidden="true">
            <path d="M3 3l6 6M9 3l-6 6" />
          </svg>
        </button>
      </div>
      <div
        className="flex items-center gap-1.5 rounded-b-xl border-t px-2.5 py-2"
        style={{ borderColor: "var(--line)", background: "var(--footer)" }}
      >
        {action && (
          <button
            type="button"
            onClick={action.run}
            className="flex h-7 items-center rounded-md px-3 text-[12px] font-medium"
            style={{ background: "var(--accent)", color: "var(--accent-fg)" }}
          >
            {action.label}
          </button>
        )}
        <span className="flex-1" />
        <span className="text-[11px]" style={{ color: "var(--muted)" }}>
          Esc to close
        </span>
      </div>
    </>
  );
}

/**
 * The answer as plain text, except the bits of Markdown models add anyway:
 * `**bold**` is shown bold, leading `#` of a heading line is dropped.
 */
function Formatted({ text }: { text: string }) {
  const cleaned = text.replace(/^#{1,6}\s+/gm, "");
  const parts = cleaned.split(/\*\*(.+?)\*\*/g);
  return (
    <>
      {parts.map((part, i) =>
        i % 2 === 1 ? (
          <b key={i} className="font-semibold">
            {part}
          </b>
        ) : (
          part
        ),
      )}
    </>
  );
}

/** "claude-haiku-5-5" → "Haiku". */
function modelName(model: string): string {
  const m = model.match(/claude-([a-z]+)/);
  return m ? m[1][0].toUpperCase() + m[1].slice(1) : model;
}
