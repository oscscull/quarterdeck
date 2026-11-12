<script lang="ts">
  import { api, errorText } from "../../lib/api";
  import { route } from "../../lib/route";
  import { t } from "../../lib/copy";
  import { dayClock } from "../../lib/time";
  import type { Urgent } from "../../lib/types";

  interface Props {
    item: Urgent;
    now: Date;
    onDone: () => void;
  }

  let { item, now, onDone }: Props = $props();

  let replying = $state(false);
  let message = $state("");
  let busy = $state(false);
  let error = $state("");

  /** Runs an action, then clears the item. Any failure stays on the row. */
  async function act(work: () => Promise<unknown>): Promise<void> {
    busy = true;
    error = "";
    try {
      await work();
      onDone();
    } catch (e) {
      error = t("urgent.failed", { error: errorText(e) });
    } finally {
      busy = false;
    }
  }

  const dismiss = () => act(() => route.dismissUrgent(item.node, item.id));
  const runNow = (id: string) =>
    act(async () => {
      await route.runSchedule(item.node, id);
      await route.dismissUrgent(item.node, item.id);
    });
  const retry = () => act(() => route.retryUrgent(item.node, item.id));
  const send = () => message.trim() && act(() => api.resolveUrgent(item.id, message.trim()));
</script>

<li class="row">
  <div class="top">
    <span class="dot"></span>
    <span class="body">
      <span class="title">{item.title}</span>
      <span class="hint detail">{item.detail}</span>
    </span>
    <span class="mono at">{dayClock(item.raisedAt, now)}</span>
  </div>
  <div class="actions">
    {#if item.scheduleId}
      {@const id = item.scheduleId}
      <button type="button" class="button-primary small" disabled={busy} onclick={() => runNow(id)}>{t("urgent.runNow")}</button>
    {/if}
    {#if item.runId}
      <button type="button" class="button-primary small" disabled={busy} onclick={retry}>{t("urgent.retry")}</button>
    {/if}
    {#if !item.node}
      <!-- Quarterdeck's Claude acts on this machine, so it can only resolve local items. -->
      <button type="button" class="button-secondary small" disabled={busy} onclick={() => (replying = !replying)}>{t("urgent.tell")}</button>
    {/if}
    <button type="button" class="quiet" disabled={busy} onclick={dismiss}>{t("urgent.dismiss")}</button>
  </div>
  {#if replying}
    <form onsubmit={(e) => { e.preventDefault(); void send(); }}>
      <label class="grow">
        <span class="sr-only">{t("urgent.messageLabel")}</span>
        <input class="input" bind:value={message} placeholder={t("urgent.messagePlaceholder")} disabled={busy} />
      </label>
      <button type="submit" class="button-primary small" disabled={busy || !message.trim()}>{t("urgent.send")}</button>
    </form>
  {/if}
  {#if busy && replying}<p class="hint indent">{t("urgent.working")}</p>{/if}
  {#if error}<p class="error indent">{error}</p>{/if}
</li>

<style>
  li {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 0;
  }

  .top {
    display: flex;
    gap: 12px;
    align-items: baseline;
  }

  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
    background: var(--urgent-dot);
    transform: translateY(-2px);
  }

  .body {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .title {
    font-weight: 500;
  }

  .detail {
    font-size: 13px;
  }

  .at {
    font-size: 12px;
    color: var(--faint);
  }

  .actions,
  form,
  .indent {
    padding-left: 19px;
  }

  .actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  form {
    display: flex;
    gap: 8px;
  }

  .grow {
    flex: 1 1 0;
    min-width: 0;
  }

  .grow input {
    width: 100%;
    font-size: 14px;
    background: var(--surface-soft);
  }

  .small {
    padding: 8px 14px;
    font-size: 13px;
  }

  .quiet {
    padding: 8px 10px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
  }
</style>
