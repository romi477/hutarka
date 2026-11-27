import { popup } from "@/lib/api";

/**
 * ⌘C, ⌘X, ⌘V, ⌘A, ⌘Z and ⇧⌘Z for the popup. Hutarka has no Edit menu (it
 * lives in the menu bar), and without one the webview ignores these keys,
 * so the page does them itself. Returns true when the key was handled.
 *
 * ⌘C with nothing selected outside the source field is left to the caller
 * (it copies the whole answer).
 */
export function handleEditKey(e: KeyboardEvent, answer: HTMLElement | null): boolean {
  if (!e.metaKey || e.altKey || e.ctrlKey) return false;
  const key = e.key.toLowerCase();
  const target = e.target as HTMLElement | null;

  if (target instanceof HTMLTextAreaElement) {
    const field = target;
    const selected = field.value.slice(field.selectionStart, field.selectionEnd);
    switch (key) {
      case "c":
      case "x":
        e.preventDefault();
        if (selected) {
          popup.copyText(selected);
          if (key === "x") insert(field, "");
        }
        return true;
      case "v":
        e.preventDefault();
        popup.clipboardText().then((text) => text && insert(field, text));
        return true;
      case "a":
        e.preventDefault();
        field.select();
        return true;
      case "z":
        e.preventDefault();
        document.execCommand(e.shiftKey ? "redo" : "undo");
        return true;
      default:
        return false;
    }
  }

  if (key === "c") {
    const selected = window.getSelection()?.toString() ?? "";
    if (!selected) return false;
    e.preventDefault();
    popup.copyText(selected);
    return true;
  }
  if (key === "a" && answer) {
    e.preventDefault();
    const range = document.createRange();
    range.selectNodeContents(answer);
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(range);
    return true;
  }
  return false;
}

/** Replaces the field's selection with `text`, keeping undo and React's onChange. */
function insert(field: HTMLTextAreaElement, text: string) {
  field.focus();
  const done = text
    ? document.execCommand("insertText", false, text)
    : document.execCommand("delete");
  if (done) return;
  // Fallback: set the value through the native setter so React sees the change.
  const { selectionStart: start, selectionEnd: end, value } = field;
  const next = value.slice(0, start) + text + value.slice(end);
  const setter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")?.set;
  setter?.call(field, next);
  field.selectionStart = field.selectionEnd = start + text.length;
  field.dispatchEvent(new Event("input", { bubbles: true }));
}
