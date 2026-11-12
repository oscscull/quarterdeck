import { api, errorText } from "./api";
import { t } from "./copy";
import { changed } from "./refresh.svelte";
import type { ChatMessage } from "./types";

/** What the current view tells the "Ask Claude" bubble about itself. */
export const chatContext = $state({ text: "" });

/**
 * The chat, kept here rather than in the bubble so switching tabs doesn't lose it. Only in memory:
 * it's gone when the app restarts.
 */
export const chat = $state({
  open: false,
  /** The larger panel, for long answers. */
  big: false,
  draft: "",
  busy: false,
  error: "",
  messages: [] as ChatMessage[],
});

export async function sendChat(page: string): Promise<void> {
  const text = chat.draft.trim();
  if (!text || chat.busy) return;
  const history = $state.snapshot(chat.messages);
  chat.messages.push({ from: "you", page, text });
  chat.draft = "";
  chat.busy = true;
  chat.error = "";
  try {
    const reply = await api.askClaude(page, chatContext.text, history, text);
    chat.messages.push({ from: "claude", page, text: reply });
    // Claude may have changed things through the Quarterdeck tools.
    changed();
  } catch (e) {
    chat.error = t("chat.failed", { error: errorText(e) });
  } finally {
    chat.busy = false;
  }
}

export function clearChat(): void {
  chat.messages = [];
  chat.error = "";
}
