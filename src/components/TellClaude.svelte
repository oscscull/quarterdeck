<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { errorText } from "../lib/api";
  import { t } from "../lib/copy";
  import { autogrow, enterSends } from "../lib/enter";
  import { changed } from "../lib/refresh.svelte";
  import { route } from "../lib/route";

  interface Props {
    itemId: string;
    node: string | undefined;
    /** The item's PR or ticket: usually the quickest way to act, so it comes first. */
    link?: string | null;
  }

  let { itemId, node, link = null }: Props = $props();

  const isPr = $derived(!!link && /github\.com\/[^/]+\/[^/]+\/pull\//.test(link));

  let text = $state("");
  let note = $state("");
  let sending = $state(false);

  async function send(): Promise<void> {
    if (!text.trim()) return;
    sending = true;
    try {
      await route.tellClaude(node, itemId, text.trim());
      text = "";
      note = t("tell.sent");
      changed();
    } catch (e) {
      note = t("ask.failed", { error: errorText(e) });
    } finally {
      sending = false;
    }
  }
</script>

<form onsubmit={(e) => { e.preventDefault(); void send(); }}>
  {#if link}
    <button type="button" class="button-primary" onclick={() => link && openUrl(link)}>{t(isPr ? "tell.openPr" : "tell.openLink")}</button>
  {/if}
  <textarea
    class="input"
    rows="1"
    bind:value={text}
    use:autogrow={text}
    onkeydown={(e) => enterSends(e, () => void send())}
    placeholder={t("tell.placeholder")}
    aria-label={t("tell.label")}
  ></textarea>
  {#if text.trim()}
    <button type="submit" class="button-primary" disabled={sending}>{t("ask.send")}</button>
  {/if}
</form>
{#if note}<p class="note">{note}</p>{/if}

<style>
  form {
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }

  .input {
    flex: 1 1 0;
    min-width: 0;
    max-height: 160px;
    font-size: 14px;
    line-height: 1.45;
    resize: none;
  }

  .note {
    margin-top: 6px;
    font-size: 13px;
    color: var(--muted);
  }
</style>
