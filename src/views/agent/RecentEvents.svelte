<script lang="ts">
  import { inArea } from "../../lib/areas.svelte";
  import { t } from "../../lib/copy";
  import { dayClock } from "../../lib/time";
  import type { FiredEvent, ScheduleView } from "../../lib/types";

  interface Props {
    events: FiredEvent[];
    schedules: ScheduleView[];
    now: Date;
  }

  let { events, schedules, now }: Props = $props();

  const SHOWN = 10;
  // Most events start nothing; by default only those that did are listed.
  let all = $state(false);
  const inView = $derived(events.filter(inArea));
  const shown = $derived((all ? inView : inView.filter((e) => e.started.length > 0)).slice(0, SHOWN));

  function startedText(e: FiredEvent): string {
    if (e.started.length === 0) return t("events.startedNothing");
    const names = e.started.map((id) => schedules.find((s) => s.id === id && s.node === e.node)?.name ?? id);
    return t("events.started", { names: names.join(", ") });
  }
</script>

{#if inView.length > 0}
  <section aria-label={t("events.title")}>
    <div class="head">
      <h2 class="section-label">{t("events.title")}</h2>
      <button type="button" class="link toggle" onclick={() => (all = !all)}>
        {all ? t("events.showStarted") : t("events.showAll", { n: inView.length })}
      </button>
    </div>
    {#if shown.length === 0}<p class="hint none">{t("events.noneStarted")}</p>{/if}
    <ul>
      {#each shown as e (`${e.node ?? ""}${e.id}`)}
        <li>
          <span class="mono when">{dayClock(e.at, now)}</span>
          <span class="body">
            <span>{e.about}</span>
            <span class="hint meta">{startedText(e)}</span>
          </span>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  section {
    margin-top: 44px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }

  .toggle,
  .none {
    font-size: 13px;
  }

  .none {
    margin-top: 8px;
  }

  ul {
    margin-top: 8px;
  }

  li {
    display: flex;
    gap: 12px;
    padding: 8px 0;
    border-top: 1px solid var(--line-soft);
    font-size: 14px;
  }

  .when {
    width: 84px;
    flex: none;
    font-size: 13px;
    color: var(--muted);
  }

  .body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-wrap: anywhere;
  }

  .meta {
    font-size: 13px;
  }
</style>
