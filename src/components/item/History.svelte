<script lang="ts">
  import { t } from "../../lib/copy";
  import { dayClock } from "../../lib/time";
  import { passText } from "../../lib/turn";
  import type { Change, ItemEvent } from "../../lib/types";

  interface Props {
    events: ItemEvent[];
    now: Date;
  }

  let { events, now }: Props = $props();
  let open = $state(false);
  let traceOpen = $state(false);

  // Records are written for people; the trace is bookkeeping. Older peers send no `trace`, so
  // only their notes count as record there.
  const isRecord = (e: ItemEvent): boolean =>
    e.trace === undefined ? e.change.kind === "note" : !e.trace;
  const records = $derived(events.filter(isRecord));
  const trace = $derived(events.filter((e) => !isRecord(e)));

  function describe(change: Change): string {
    switch (change.kind) {
      case "created":
        return t("history.created");
      case "stage":
        return t("history.stage", { stage: t(`stage.${change.stage}`) });
      case "status":
        return change.status
          ? t("history.status", { status: t(`status.${change.status}`) })
          : t("history.statusCleared");
      case "label":
        return t("history.label", { label: change.label });
      case "note":
        return t("history.note", { text: change.text });
      case "record":
        return t("history.record", { text: change.text });
      case "turn":
        return passText(change.turn, change.why);
    }
  }
</script>

{#if events.length > 0}
  <section aria-label={t("item.history")}>
    <button type="button" class="link toggle" aria-expanded={open} onclick={() => (open = !open)}>
      {open ? t("item.hideHistory") : t("item.showHistory", { n: records.length })}
    </button>
    {#if open}
      {#if records.length === 0}
        <p class="hint none">{t("history.noRecord")}</p>
      {/if}
      <ol>
        {#each records as e (e.id)}
          <li>
            <span class="mono when">{dayClock(e.at, now)}</span>
            <span class="body">
              {#if e.change.kind === "record"}<span class="tag tag-{e.change.tag}">{t(`history.tag.${e.change.tag}`)}</span>{/if}
              <span>{describe(e.change)}</span>
            </span>
          </li>
        {/each}
      </ol>
      {#if trace.length > 0}
        <button type="button" class="link toggle trace-toggle" aria-expanded={traceOpen} onclick={() => (traceOpen = !traceOpen)}>
          {traceOpen ? t("history.hideTrace") : t("history.showTrace", { n: trace.length })}
        </button>
        {#if traceOpen}
          <ol class="trace">
            {#each trace as e (e.id)}
              <li>
                <span class="mono when">{dayClock(e.at, now)}</span>
                <span>{describe(e.change)}</span>
              </li>
            {/each}
          </ol>
        {/if}
      {/if}
    {/if}
  </section>
{/if}

<style>
  .toggle {
    font-size: 13px;
  }

  .none {
    margin-top: 6px;
    font-size: 13px;
  }

  ol {
    margin-top: 6px;
    max-height: 320px;
    overflow-y: auto;
  }

  li {
    display: flex;
    gap: 12px;
    padding: 6px 0;
    font-size: 14px;
    color: var(--ink);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .tag {
    align-self: flex-start;
    font-size: 11px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--chip-queued-bg);
    color: var(--chip-queued-fg);
  }

  .tag-why {
    background: var(--chip-review-bg);
    color: var(--chip-review-fg);
  }

  .tag-delivered {
    background: var(--chip-approved-bg);
    color: var(--chip-approved-fg);
  }

  .tag-blocked,
  .tag-correction {
    background: var(--chip-decision-bg);
    color: var(--chip-decision-fg);
  }

  .trace-toggle {
    margin-top: 8px;
  }

  .trace li {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--muted);
  }

  .when {
    width: 84px;
    flex: none;
    font-size: 12px;
    color: var(--muted);
  }
</style>
