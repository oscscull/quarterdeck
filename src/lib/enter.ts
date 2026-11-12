/**
 * Keys for a message box: Enter sends, Cmd/Ctrl/Shift+Enter adds a new line. A textarea only adds
 * one for Shift+Enter by itself, so the others are inserted here.
 */
export function enterSends(event: KeyboardEvent, send: () => void): void {
  if (event.key !== "Enter" || event.isComposing) return;
  if (event.shiftKey) return;
  event.preventDefault();
  if (!event.metaKey && !event.ctrlKey) {
    send();
    return;
  }
  const box = event.currentTarget;
  if (!(box instanceof HTMLTextAreaElement)) return;
  box.setRangeText("\n", box.selectionStart, box.selectionEnd, "end");
  box.dispatchEvent(new Event("input", { bubbles: true }));
}

/** Grows a textarea with its text, up to its CSS `max-height`. Pass the bound value to re-measure. */
export function autogrow(box: HTMLTextAreaElement, _value: string): { update: () => void } {
  const fit = (): void => {
    box.style.height = "auto";
    // scrollHeight leaves out the border, which border-box sizing counts.
    box.style.height = `${box.scrollHeight + box.offsetHeight - box.clientHeight}px`;
  };
  fit();
  return { update: fit };
}
