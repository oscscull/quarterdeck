<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { t } from "../../lib/copy";
  import { dayClock } from "../../lib/time";
  import type { Reference } from "../../lib/types";

  interface Props {
    reference: Reference;
    n: number;
    now: Date;
    onClose: () => void;
  }

  let { reference, n, now, onClose }: Props = $props();
</script>

<div class="card" role="dialog" aria-label={t("briefing.cite", { n })}>
  <div class="head">
    <span class="kind" class:you={reference.kind === "you_said"}>{t(`briefing.kind.${reference.kind}`)}</span>
    <span class="title">{reference.title}</span>
    <button type="button" class="close" aria-label={t("briefing.close")} onclick={onClose}>×</button>
  </div>
  <p class="quote">{reference.quote}</p>
  <p class="meta">
    {t(`briefing.refBy.${reference.by}`)} · {dayClock(reference.at, now)}
    {#if reference.items.length > 0}· {t("briefing.refOn", { items: reference.items.join(", ") })}{/if}
    {#if reference.url}
      · <button type="button" class="link" onclick={() => reference.url && openUrl(reference.url)}>{t("briefing.refOpen")}</button>
    {/if}
  </p>
</div>

<style>
  .card {
    margin: 4px 0 18px;
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: 0 6px 24px rgb(25 25 23 / 8%);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .kind {
    flex: none;
    font-size: 11px;
    font-weight: 500;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--chip-queued-bg);
    color: var(--chip-queued-fg);
  }

  .kind.you {
    background: var(--chip-decision-bg);
    color: var(--chip-decision-fg);
  }

  .title {
    flex: 1;
    min-width: 0;
    font-size: 14px;
    font-weight: 500;
  }

  .close {
    border: 0;
    background: none;
    font-size: 18px;
    line-height: 1;
    color: var(--muted);
  }

  .quote {
    padding-left: 10px;
    border-left: 2px solid var(--line);
    font-size: 14px;
    line-height: 1.5;
  }

  .meta {
    font-size: 12px;
    color: var(--muted);
  }

  .meta .link {
    font-size: 12px;
  }
</style>
