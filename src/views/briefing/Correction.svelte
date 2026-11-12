<script lang="ts">
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/copy";
  import { autogrow, enterSends } from "../../lib/enter";
  import { changed } from "../../lib/refresh.svelte";
  import type { ChatMessage, Correction } from "../../lib/types";

  interface Props {
    /** What the user is looking at, in words, for Claude. */
    scope: string;
    label: string;
    area: string | null;
    project: string | null;
    /** The machine holding what's shown, when it isn't this one. */
    node: string | undefined;
  }

  let { scope, label, area, project, node }: Props = $props();

  // Kept only while the tab is open: what lasts is saved as references and item changes.
  let messages = $state<ChatMessage[]>([]);
  let draft = $state("");
  let busy = $state(false);
  let error = $state("");

  const PAGE = "Briefing";

  async function send(): Promise<void> {
    const message = draft.trim();
    if (!message || busy) return;
    const correction: Correction = { scope, area, project, history: $state.snapshot(messages), message };
    messages.push({ from: "you", page: PAGE, text: message });
    draft = "";
    busy = true;
    error = "";
    try {
      const reply = node
        ? await api.peerCall<string>(node, "correct_briefing", correction)
        : await api.correctBriefing(correction);
      messages.push({ from: "claude", page: PAGE, text: reply });
      changed();
    } catch (e) {
      error = t("briefing.correct.failed", { error: errorText(e) });
    } finally {
      busy = false;
    }
  }
</script>

<section class="box" aria-label={label}>
  {#if messages.length > 0}
    <ol class="thread">
      {#each messages as m, i (i)}
        <li class:you={m.from === "you"}>
          <span class="who">{t(m.from === "you" ? "briefing.correct.you" : "briefing.correct.agent")}</span>
          <span class="text">{m.text}</span>
        </li>
      {/each}
      {#if busy}<li class="thinking">{t("briefing.correct.thinking")}</li>{/if}
    </ol>
  {/if}
  <form onsubmit={(e) => { e.preventDefault(); void send(); }}>
    <label for="correct" class="label">{label}</label>
    <textarea
      id="correct"
      class="input"
      rows="2"
      bind:value={draft}
      use:autogrow={draft}
      onkeydown={(e) => enterSends(e, () => void send())}
      placeholder={t("briefing.correct.placeholder")}
    ></textarea>
    <div class="foot">
      <span class="hint">{t("briefing.correct.hint")}</span>
      <span class="actions">
        {#if messages.length > 0 && !busy}
          <button type="button" class="link" onclick={() => (messages = [])}>{t("briefing.correct.clear")}</button>
        {/if}
        <button type="submit" class="button-primary" disabled={busy || !draft.trim()}>{t("ask.send")}</button>
      </span>
    </div>
  </form>
  {#if error}<p class="error">{error}</p>{/if}
</section>

<style>
  .box {
    margin-top: 32px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
  }

  .thread {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 420px;
    overflow-y: auto;
  }

  .thread li {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--surface-soft);
    font-size: 14px;
    line-height: 1.5;
  }

  .thread li.you {
    align-self: flex-end;
    max-width: 85%;
    background: var(--ground);
  }

  .who {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }

  .text {
    white-space: pre-wrap;
  }

  .thinking {
    color: var(--muted);
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .label {
    font-size: 13px;
    font-weight: 600;
  }

  textarea {
    resize: none;
    max-height: 240px;
  }

  .foot {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .foot .hint {
    font-size: 12px;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }
</style>
