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
  const shown = $derived(events.filter(inArea).slice(0, SHOWN));

  function startedText(e: FiredEvent): string {
    if (e.started.length === 0) return t("events.startedNothing");
    const names = e.started.map((id) => schedules.find((s) => s.id === id && s.node === e.node)?.name ?? id);
    return t("events.started", { names: names.join(", ") });
  }
</script>

{#if shown.length > 0}
  <section aria-label={t("events.title")}>
    <h2 class="section-label">{t("events.title")}</h2>
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
