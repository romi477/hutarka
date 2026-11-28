import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { popup, type Lang, type Snapshot } from "@/lib/api";
import { TargetMenu } from "./TargetMenu";
import { SourceCell } from "./SourceCell";
import { handleEditKey } from "./editKeys";

/** Room around the card for its shadow (window 560 = card 520 + 2 × 20). */
const MARGIN = 20;
/** Window widths: normal, and for long text so it needs less scrolling. */
const WIDTH = 560;
const WIDE_WIDTH = 820;
/** Original plus answer longer than this opens the wide window. */
const WIDE_FROM_CHARS = 1200;
const MENU_HEIGHT = 170;

const ERROR_TITLES: Record<string, string> = {
  no_key: "No Anthropic API key",
  not_trusted: "Accessibility permission needed",
  too_long: "Text too long",
  api: "The translation failed",
};

const ERROR_HINTS: Record<string, string> = {
  no_key:
    "Add your key in Settings › General to translate. It is stored in app settings, not in ~/Hutarka.",
  not_trusted:
    "Allow Hutarka in System Settings › Privacy & Security › Accessibility, then try again.",
};

function Arrow() {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M2 6h8M7 3l3 3-3 3" />
    </svg>
  );
}

export function Popup() {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [menu, setMenu] = useState(false);
  /** The source text as edited in the popup. */
  const [draft, setDraft] = useState("");
  const [open, setOpen] = useState(false);
  const [inputError, setInputError] = useState("");
  /** The next lookup comes from our own Translate: keep the source cell as it is. */
  const ownRun = useRef(false);
  const wrapper = useRef<HTMLDivElement>(null);
  const answer = useRef<HTMLDivElement>(null);

  // A new lookup: show its text in the source cell, collapsed for a selection,
  // open (to type) for an empty popup.
  const lookupId = snap?.id;
  useEffect(() => {
    if (!snap) return;
    setDraft(snap.text);
    setInputError("");
    if (ownRun.current) ownRun.current = false;
    else setOpen(!snap.from_selection);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lookupId]);

  // Pinned: the popup stays open when it loses focus and follows across Spaces.
  const [pinned, setPinned] = useState(false);
  useEffect(() => {
    popup.pinned().then(setPinned);
    const off = popup.onPinned(setPinned);
    return () => {
      off.then((f) => f());
    };
  }, []);

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

  const width =
    snap && snap.text.length + snap.result.length > WIDE_FROM_CHARS ? WIDE_WIDTH : WIDTH;

  // The window follows the card's height. The card exists only once the first
  // snapshot arrived, so the observer is attached then (not on mount).
  const hasSnap = snap !== null;
  useLayoutEffect(() => {
    const el = wrapper.current;
    if (!el) return;
    const send = () =>
      popup.resize(Math.ceil(el.offsetHeight + (menu ? MENU_HEIGHT : 0)), width);
    send();
    const observer = new ResizeObserver(send);
    observer.observe(el);
    return () => observer.disconnect();
  }, [menu, hasSnap, width]);

  /**
   * Runs the lookup again into `to`: the edited source text if it changed,
   * otherwise the same text; with no text yet, only remembers the choice.
   */
  const run = useCallback(
    (to: Lang, explain: boolean) => {
      if (!snap) return;
      setMenu(false);
      const edited = draft.trim() !== "" && draft !== snap.text;
      if (edited) {
        ownRun.current = true;
        popup
          .translate(draft, to, explain, snap.from_selection)
          .catch((e) => {
            ownRun.current = false;
            setInputError(String(e));
          });
      } else if (snap.text) {
        popup.rerun(to, explain);
      } else {
        // Nothing to translate yet: keep the choice for the first translation.
        popup.choose(to, explain);
      }
    },
    [snap, draft],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!snap) return;
      if (handleEditKey(e, answer.current)) return;
      const typing = (e.target as HTMLElement | null)?.tagName === "TEXTAREA";
      if (typing && e.key === "Enter" && e.metaKey) {
        e.preventDefault();
        run(snap.to, snap.explain);
        return;
      }
      // Inside the source field, plain keys and ⌘C / ⌘V edit text.
      if (
        typing &&
        e.key !== "Escape" &&
        !(e.metaKey && "ep123456789".includes(e.key.toLowerCase()))
      )
        return;
      if (e.key === "Escape") {
        e.preventDefault();
        if (menu) setMenu(false);
        else popup.close();
      } else if (e.metaKey && e.key.toLowerCase() === "c" && snap.status === "done") {
        // Nothing selected (handleEditKey copies a selection): the whole answer.
        e.preventDefault();
        popup.copy();
      } else if (
        e.key === "Enter" &&
        snap.status === "done" &&
        snap.from_selection
      ) {
        e.preventDefault();
        popup.replace();
      } else if (e.metaKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        popup.pin(!pinned);
      } else if (e.metaKey && e.key.toLowerCase() === "e") {
        e.preventDefault();
        run(snap.to, !snap.explain);
      } else if (e.metaKey && /^[1-9]$/.test(e.key)) {
        e.preventDefault();
        const target = snap.targets[Number(e.key) - 1];
        if (target) run(target, snap.explain);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [snap, menu, run, draft, pinned]);

  if (!snap) return null;

  const streaming = snap.status === "streaming";
  const done = snap.status === "done";
  const showsWord =
    snap.from_selection &&
    snap.kind !== "text" &&
    snap.text.trim().length <= 60;
  const hasAnswer = snap.result !== "" || streaming || done;

  return (
    <div ref={wrapper} style={{ padding: MARGIN }}>
      <div
        className="relative flex flex-col rounded-xl border"
        style={{
          // Never taller than the screen: only the answer scrolls, so the
          // header, the original and the buttons stay reachable.
          maxHeight: window.screen.availHeight - 2 * MARGIN,
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
                if (
                  (e.target as HTMLElement).hasAttribute(
                    "data-tauri-drag-region",
                  )
                )
                  popup.dragged();
              }}
              className="flex shrink-0 cursor-grab items-center gap-2 pt-2.5 pr-2.5 pl-3.5"
            >
              {showsWord && (
                <span
                  data-tauri-drag-region
                  className="truncate text-[17px] font-semibold"
                  style={{ maxWidth: 200 }}
                >
                  {snap.text.trim()}
                </span>
              )}
              <div className="relative">
                <button
                  type="button"
                  aria-label="Change target language"
                  aria-expanded={menu}
                  onClick={() => setMenu(!menu)}
                  className="flex h-6 items-center gap-1 rounded-md px-2 text-[12px] font-medium"
                  style={{
                    background: menu ? "var(--chip-strong)" : "var(--chip)",
                    color: "var(--fg-2)",
                  }}
                >
                  <Arrow />
                  {snap.to.toUpperCase()}
                  <svg
                    width="10"
                    height="10"
                    viewBox="0 0 10 10"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.4"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <path d="M2.5 4l2.5 2.5L7.5 4" />
                  </svg>
                </button>
                {menu && (
                  <TargetMenu
                    current={snap.to}
                    targets={snap.targets}
                    onPick={(d) => run(d, snap.explain)}
                    onEditPrompts={() => popup.openSettings("translator")}
                  />
                )}
              </div>
              <label
                className="flex h-6 items-center gap-1.5 px-1 text-[12px]"
                title="Explain (⌘E)"
              >
                <input
                  type="checkbox"
                  checked={snap.explain}
                  onChange={() =>
                    run(snap.to, !snap.explain)
                  }
                  className="m-0 h-3.5 w-3.5"
                />
                Explain
              </label>
              <label
                className="flex h-6 items-center gap-1.5 px-1 text-[12px]"
                title="Keep open when it loses focus, on every Space (⌘P)"
              >
                <input
                  type="checkbox"
                  checked={pinned}
                  onChange={() => popup.pin(!pinned)}
                  className="m-0 h-3.5 w-3.5"
                />
                Pin
              </label>
              <span data-tauri-drag-region className="flex-1 self-stretch" />
              <span
                data-tauri-drag-region
                className="flex items-center gap-1.5 text-[11px]"
                style={{ color: "var(--muted)" }}
              >
                {streaming ? (
                  <>
                    <svg
                      className="animate-spin"
                      width="12"
                      height="12"
                      viewBox="0 0 12 12"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.5"
                      strokeLinecap="round"
                      aria-hidden="true"
                    >
                      <path d="M6 1.5a4.5 4.5 0 1 1-4.5 4.5" />
                    </svg>
                    {snap.kind === "explain" ? "Explaining…" : "Translating…"}
                  </>
                ) : (
                  done &&
                  `${modelName(snap.model)} · ${(snap.ms / 1000).toFixed(1)} s`
                )}
              </span>
              <button
                type="button"
                aria-label="Close"
                onClick={() => popup.close()}
                className="flex h-6 w-6 items-center justify-center rounded-md"
                style={{ color: "var(--muted)" }}
              >
                <svg
                  width="12"
                  height="12"
                  viewBox="0 0 12 12"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  strokeLinecap="round"
                  aria-hidden="true"
                >
                  <path d="M3 3l6 6M9 3l-6 6" />
                </svg>
              </button>
            </div>
            <SourceCell
              key={snap.id}
              text={draft}
              open={open}
              focus={open && !snap.from_selection}
              error={inputError}
              onToggle={() => setOpen(!open)}
              onChange={(t) => {
                setInputError("");
                setDraft(t);
              }}
              onTranslate={() => run(snap.to, snap.explain)}
              onClear={() => {
                setDraft("");
                setInputError("");
                popup.clear();
              }}
            />
            {hasAnswer && (
              <div
                ref={answer}
                className="selectable min-h-0 overflow-y-auto px-4 pt-2 pb-3.5 text-[14px] leading-normal whitespace-pre-wrap"
              >
                <Formatted text={snap.result} />
                {streaming && (
                  <span
                    className="ml-0.5 inline-block h-4 w-0.5 animate-pulse align-[-3px]"
                    style={{ background: "var(--accent)" }}
                  />
                )}
              </div>
            )}
            <div
              className="flex shrink-0 items-center gap-1.5 rounded-b-xl border-t px-2.5 py-2"
              style={{
                borderColor: "var(--line)",
                background: "var(--footer)",
              }}
            >
              <button
                type="button"
                disabled={!done}
                onClick={() => popup.copy()}
                className="btn flex h-7 items-center gap-1.5 rounded-md px-2.5 text-[12px]"
              >
                Copy{" "}
                <span className="text-[11px]" style={{ color: "var(--muted)" }}>
                  ⌘C
                </span>
              </button>
              {snap.from_selection && (
                <button
                  type="button"
                  disabled={!done}
                  onClick={() => popup.replace()}
                  className="btn-primary flex h-7 items-center gap-1.5 rounded-md px-2.5 text-[12px]"
                >
                  Replace <span className="text-[11px] font-normal opacity-70">↵</span>
                </button>
              )}
              <span className="flex-1" />
              {/* Same as the header's ×, within reach after scrolling a long answer. */}
              <button
                type="button"
                onClick={() => popup.close()}
                className="btn-quiet h-7 rounded-md px-2.5 text-[12px]"
              >
                Close
              </button>
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
      ? { label: "Open Settings", run: () => popup.openSettings("general") }
      : kind === "not_trusted"
        ? {
            label: "Open Accessibility Settings",
            run: () => popup.openAccessibility(),
          }
        : null;
  return (
    <>
      <div className="flex items-start gap-2.5 pt-3.5 pr-2.5 pb-3 pl-4">
        <svg
          width="18"
          height="18"
          viewBox="0 0 18 18"
          fill="none"
          stroke="var(--warn)"
          strokeWidth="1.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          role="img"
          aria-label="Warning"
          className="mt-px shrink-0"
        >
          <path d="M9 2.5l7 12.5H2z" />
          <path d="M9 7.5v3.5M9 13.2v.1" />
        </svg>
        <div className="flex flex-1 flex-col gap-1">
          <span className="font-semibold">{ERROR_TITLES[kind]}</span>
          <span
            className="selectable leading-snug"
            style={{ color: "var(--fg-2)" }}
          >
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
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
            aria-hidden="true"
          >
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
            className="btn-primary flex h-7 items-center rounded-md px-3 text-[12px]"
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
