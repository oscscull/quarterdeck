<script lang="ts">
  import { untrack } from "svelte";

  import { api } from "../../lib/api";
  import { t } from "../../lib/copy";
  import type { EventKindInfo, Trigger } from "../../lib/types";
  import TriggerEditor from "./TriggerEditor.svelte";

  interface Props {
    triggers: Trigger[];
    now: Date;
    onChange: (triggers: Trigger[]) => void;
  }

  let { triggers, now, onChange }: Props = $props();

  // Each row keeps its own id, so removing one doesn't hand its typing to the next.
  let nextId = 0;
  const row = (trigger: Trigger) => ({ id: nextId++, trigger });
  let rows = $state(untrack(() => triggers.map(row)));
  let kinds = $state<EventKindInfo[]>([]);

  $effect(() => {
    api.eventKinds().then((k) => (kinds = k)).catch(() => (kinds = []));
  });

  function set(id: number, trigger: Trigger): void {
    rows = rows.map((r) => (r.id === id ? { id, trigger } : r));
    onChange(rows.map((r) => r.trigger));
  }

  function remove(id: number): void {
    rows = rows.filter((r) => r.id !== id);
    onChange(rows.map((r) => r.trigger));
  }

  function add(): void {
    rows = [...rows, row({ type: "cron", expr: "0 9 * * 1-5" })];
    onChange(rows.map((r) => r.trigger));
  }
</script>

<div>
  <h3 class="field">{t("schedule.trigger")}</h3>
  {#if rows.length === 0}<p class="hint small">{t("schedule.byHandOnly")}</p>{/if}
  {#each rows as r (r.id)}
    <TriggerEditor trigger={r.trigger} {kinds} {now} onChange={(next) => set(r.id, next)} onRemove={() => remove(r.id)} />
  {/each}
  <button type="button" class="button-secondary" onclick={add}>{t("schedule.addTrigger")}</button>
</div>

<style>
  div {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  h3 {
    margin: 0 0 4px;
  }

  button {
    align-self: flex-start;
    margin-top: 6px;
  }

  .small {
    font-size: 13px;
  }
</style>
