import { useEffect, useRef } from "react";

const MAX_CHARS = 20_000;
const MAX_FIELD_HEIGHT = 180;

interface Props {
  text: string;
  open: boolean;
  /** Put the caret in the field (an empty popup waiting for typing). */
  focus: boolean;
  error: string;
  onToggle: () => void;
  onChange: (text: string) => void;
  onTranslate: () => void;
  /** Empties the original and the answer. */
  onClear: () => void;
}

function Chevron({ open }: { open: boolean }) {
  return (
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
      style={{ transform: open ? "rotate(90deg)" : undefined, transition: "transform 120ms" }}
    >
      <path d="M3.5 2l3 3-3 3" />
    </svg>
  );
}

/**
 * The text being translated. Collapsed: one dimmed line. Open: an editable
 * field; ⌘↵ or Translate sends what is in it, Clear empties it and the answer.
 */
export function SourceCell({
  text,
  open,
  focus,
  error,
  onToggle,
  onChange,
  onTranslate,
  onClear,
}: Props) {
  const field = useRef<HTMLTextAreaElement>(null);

  // The field grows with its text, up to a limit, then scrolls.
  useEffect(() => {
    const el = field.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, MAX_FIELD_HEIGHT)}px`;
  }, [text, open]);

  useEffect(() => {
    if (open && focus) field.current?.focus();
  }, [open, focus]);

  return (
    <div className="mx-3 mt-2 rounded-lg border" style={{ borderColor: "var(--line)" }}>
      <button
        type="button"
        aria-expanded={open}
        onClick={onToggle}
        className="flex w-full items-center gap-1.5 px-2.5 py-1.5 text-left text-[12px]"
        style={{ color: "var(--muted)" }}
      >
        <Chevron open={open} />
        {open ? (
          <span>Original</span>
        ) : (
          <span className="truncate" style={{ color: text ? "var(--fg-2)" : "var(--muted)" }}>
            {text || "Type text to translate"}
          </span>
        )}
      </button>
      {open && (
        <div className="flex flex-col gap-1.5 px-2.5 pb-2">
          <textarea
            ref={field}
            value={text}
            maxLength={MAX_CHARS}
            rows={2}
            placeholder="Type or paste text to translate"
            onChange={(e) => onChange(e.target.value)}
            className="selectable w-full resize-none rounded-md border px-2 py-1.5 text-[13px] leading-normal outline-none"
            style={{ background: "var(--field)", borderColor: "var(--border)" }}
          />
          <div className="flex items-center gap-2">
            <button
              type="button"
              disabled={!text.trim()}
              onClick={onTranslate}
              title="Translate (⌘↵)"
              className="flex h-6 shrink-0 items-center gap-1.5 whitespace-nowrap rounded-md px-2.5 text-[12px] font-medium disabled:opacity-50"
              style={{ background: "var(--accent)", color: "var(--accent-fg)" }}
            >
              Translate
              <span className="text-[11px] font-normal opacity-75">⌘↵</span>
            </button>
            {error && (
              <span className="min-w-0 truncate text-[11px]" style={{ color: "var(--warn)" }}>
                {error}
              </span>
            )}
            <span className="flex-1" />
            <button
              type="button"
              onClick={onClear}
              className="flex h-6 shrink-0 items-center rounded-md px-2.5 text-[12px] hover:bg-[var(--chip)] disabled:opacity-50"
              style={{ color: "var(--muted)" }}
            >
              Clear
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
