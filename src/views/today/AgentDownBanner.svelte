<script lang="ts">
  import { api, errorText } from "../../lib/api";
  import { t } from "../../lib/copy";
  import { clock } from "../../lib/time";
  import type { Outage } from "../../lib/types";

  interface Props {
    outage: Outage | null;
    onChanged: () => void;
  }

  let { outage, onChanged }: Props = $props();

  let busy = $state(false);
  /** What the last check found, shown until the next one. */
  let result = $state("");
  let back = $state(false);

  async function check(): Promise<void> {
    busy = true;
    result = "";
    try {
      const still = await api.checkAgent();
      back = !still;
      result = still ? t("outage.still", { why: still.why }) : t("outage.back");
      onChanged();
    } catch (e) {
      result = t("outage.failed", { error: errorText(e) });
    } finally {
      busy = false;
    }
  }
</script>

{#if outage}
  <div class="banner down" role="alert">
    <span class="text">
      <span class="title">{t("outage.title")}</span>
      <span class="sub">{outage.why}</span>
      <span class="sub">
        {outage.nextCheck ? t("outage.waiting", { when: clock(outage.nextCheck) }) : t("outage.waitingSoon")}
      </span>
      {#if busy}
        <span class="sub">{t("outage.checking")}</span>
      {:else if result}
        <span class="sub result">{result}</span>
      {/if}
    </span>
    <button type="button" class="button-primary" disabled={busy} onclick={check}>{t("outage.check")}</button>
  </div>
{:else if back}
  <div class="banner" role="status">
    <span class="text"><span class="title">{result}</span></span>
    <button type="button" class="quiet" onclick={() => (back = false)}>{t("outage.dismiss")}</button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
    padding: 14px 16px;
    border-radius: var(--radius);
    background: var(--paused-bg);
  }

  .down {
    background: var(--urgent-bg);
  }

  .text {
    flex: 1 1 280px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .title {
    font-weight: 500;
  }

  .sub {
    font-size: 13px;
    color: #4a4843;
    overflow-wrap: anywhere;
  }

  .result {
    font-weight: 500;
  }

  .quiet {
    padding: 8px 10px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
  }
</style>
