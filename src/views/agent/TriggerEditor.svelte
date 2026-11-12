<script lang="ts">
  import { untrack } from "svelte";

  import { t } from "../../lib/copy";
  import type { EventKindInfo, Trigger } from "../../lib/types";
  import CronExplain from "./CronExplain.svelte";
  import { eventText } from "./when";

  interface Props {
    trigger: Trigger;
    kinds: EventKindInfo[];
    now: Date;
    onChange: (trigger: Trigger) => void;
    onRemove: () => void;
  }

  let { trigger, kinds, now, onChange, onRemove }: Props = $props();

  const CUSTOM = "custom.";
  /** Fields every event has. */
  const COMMON = ["area", "subject"];

  // Keep both shapes so switching back and forth doesn't lose what was typed.
  const start = untrack(() => trigger);
  const startOn = start.type === "event" ? start.on : "github.pr_new_comments";
  let mode = $state(start.type);
  let expr = $state(start.type === "cron" ? start.expr : "0 9 * * 1-5");
  let picked = $state(startOn.startsWith(CUSTOM) ? "custom" : startOn);
  let customName = $state(startOn.startsWith(CUSTOM) ? startOn.slice(CUSTOM.length) : "");
  let when = $state<Record<string, string>>(start.type === "event" ? { ...(start.when ?? {}) } : {});
  let cooldown = $state(start.type === "event" ? (start.cooldown ?? 0) : 0);

  const on = $derived(picked === "custom" ? `${CUSTOM}${customName.trim()}` : picked);
  const fields = $derived([...(kinds.find((k) => k.name === on)?.fields ?? []), ...COMMON]);

  $effect(() => {
    if (mode === "cron") {
      onChange({ type: "cron", expr });
      return;
    }
    const filters = Object.fromEntries(Object.entries(when).filter(([k, v]) => v.trim() && fields.includes(k)));
    onChange({ type: "event", on, when: filters, cooldown: Math.max(0, Math.round(cooldown || 0)) });
  });
</script>

<fieldset>
  <div class="modes">
    <label><input type="radio" bind:group={mode} value="cron" /> {t("schedule.onSchedule")}</label>
    <label><input type="radio" bind:group={mode} value="event" /> {t("schedule.onEvent")}</label>
    <button type="button" class="remove" aria-label={t("schedule.removeTrigger")} onclick={onRemove}>×</button>
  </div>

  {#if mode === "cron"}
    <label class="field">
      {t("schedule.cron")}
      <input class="input mono" bind:value={expr} spellcheck="false" />
    </label>
    <CronExplain {expr} {now} />
  {:else}
    <label class="field">
      {t("schedule.event")}
      <select class="input" bind:value={picked}>
        {#each kinds as k (k.name)}<option value={k.name}>{eventText(k.name)}</option>{/each}
        <option value="custom">{t("event.custom")}</option>
      </select>
    </label>
    {#if picked === "custom"}
      <label class="field">
        {t("schedule.customName")}
        <span class="custom"><span class="mono hint">{CUSTOM}</span><input class="input mono" bind:value={customName} spellcheck="false" /></span>
      </label>
    {/if}
    <div class="filters">
      {#each fields as f (f)}
        <label class="field small">
          {t("schedule.whenField", { field: f })}
          <input class="input mono" placeholder={t("schedule.any")} bind:value={when[f]} spellcheck="false" />
        </label>
      {/each}
      <label class="field small">
        {t("schedule.cooldown")}
        <input class="input" type="number" min="0" bind:value={cooldown} />
      </label>
    </div>
    <p class="hint small">{t("schedule.eventHint")}</p>
  {/if}
</fieldset>

<style>
  fieldset {
    margin: 0;
    padding: 12px 0;
    border: 0;
    border-top: 1px solid var(--line-soft);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .modes {
    display: flex;
    align-items: center;
    gap: 20px;
    font-size: 14px;
  }

  .remove {
    margin-left: auto;
    width: 28px;
    height: 28px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    font-size: 18px;
  }

  .remove:hover {
    background: var(--surface-soft);
  }

  .custom {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .filters {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 10px;
  }

  .small {
    font-size: 13px;
  }
</style>
