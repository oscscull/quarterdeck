<script lang="ts">
  import AreaDot from "../../components/AreaDot.svelte";
  import Toggle from "../../components/Toggle.svelte";
  import { findArea } from "../../lib/areas.svelte";
  import { t } from "../../lib/copy";
  import { dayClock } from "../../lib/time";
  import type { ScheduleView } from "../../lib/types";
  import { whenText } from "./when";

  interface Props {
    schedules: ScheduleView[];
    now: Date;
    /** Every schedule is paused; individual switches are kept as they are. */
    paused: boolean;
    onOpen: (schedule: ScheduleView) => void;
    onToggle: (schedule: ScheduleView, enabled: boolean) => void;
    onDelete: (schedule: ScheduleView) => void;
  }

  let { schedules, now, paused, onOpen, onToggle, onDelete }: Props = $props();

  function nextText(s: ScheduleView): string {
    if (!s.enabled || paused) return t("agent.paused");
    const next = s.explanation?.nextRuns[0];
    return next ? dayClock(next, now) : t("agent.onEvent");
  }

  /** "Missed 09:00 · machine asleep" when the last run didn't happen or went wrong. */
  function problemText(s: ScheduleView): string {
    const last = s.runs[0];
    if (paused || !last || (last.outcome !== "missed" && last.outcome !== "failed")) return "";
    const vars = { time: dayClock(last.startedAt, now), reason: last.summary ?? "" };
    return t(last.outcome === "missed" ? "agent.missed" : "agent.failed", vars);
  }

  const lastText = (s: ScheduleView): string =>
    s.runs[0]?.summary ? t("agent.last", { summary: s.runs[0].summary }) : t("agent.neverRun");
</script>

<ul>
  {#each schedules as s (`${s.node ?? ""}${s.id}`)}
    <li class="row" class:paused={!s.enabled || paused}>
      <button type="button" class="open" onclick={() => onOpen(s)}>
        <span class="body">
          <span class="name-row"><AreaDot area={s.area} /><span class="name">{s.name}</span></span>
          <span class="hint meta">
            {whenText(s.triggers, s.explanation)} · {findArea(s.area)?.name ?? t("area.unknown")}
          </span>
          <span class="hint meta">{lastText(s)}</span>
          {#if s.needsApproval}<span class="problem">{t("script.needsApproval")}</span>{/if}
          {#if problemText(s)}<span class="problem">{problemText(s)}</span>{/if}
        </span>
        <span class="mono next">{nextText(s)}</span>
      </button>
      <Toggle
        on={s.enabled}
        label={t(s.enabled ? "agent.pause" : "agent.resume", { name: s.name })}
        onChange={(on) => onToggle(s, on)}
      />
      <button type="button" class="delete" aria-label={t("agent.delete", { name: s.name })} onclick={() => onDelete(s)}>×</button>
    </li>
  {/each}
</ul>

<style>
  li {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .paused {
    opacity: 0.55;
  }

  .open {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 0;
    border: 0;
    background: transparent;
    text-align: left;
  }

  .body {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .name {
    font-weight: 500;
  }

  .name-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .meta {
    font-size: 13px;
  }

  .problem {
    align-self: flex-start;
    margin-top: 3px;
    padding: 3px 9px;
    border-radius: 999px;
    background: var(--urgent-bg);
    color: var(--urgent-fg);
    font-size: 12px;
    font-weight: 500;
  }

  .delete {
    width: 32px;
    height: 32px;
    flex: none;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    font-size: 18px;
  }

  .delete:hover {
    background: var(--surface-soft);
    color: var(--ink);
  }

  .next {
    font-size: 13px;
    color: var(--muted);
    text-align: right;
  }
</style>
