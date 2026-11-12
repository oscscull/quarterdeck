<script lang="ts">
  import LoadError from "../components/LoadError.svelte";
  import { api, errorText } from "../lib/api";
  import { inArea } from "../lib/areas.svelte";
  import { chatContext } from "../lib/chat.svelte";
  import { t } from "../lib/copy";
  import { poll } from "../lib/poll";
  import { mergeAgent } from "../lib/merge";
  import { fromPeers } from "../lib/peers.svelte";
  import { route } from "../lib/route";
  import type { AgentView, Schedule, ScheduleView } from "../lib/types";
  import ConnectClaude from "./agent/ConnectClaude.svelte";
  import Connections from "./agent/Connections.svelte";
  import DeleteScheduleModal from "./agent/DeleteScheduleModal.svelte";
  import LiveRuns from "./agent/LiveRuns.svelte";
  import NewScheduleModal from "./agent/NewScheduleModal.svelte";
  import PauseModal from "./agent/PauseModal.svelte";
  import PausedBanner from "./agent/PausedBanner.svelte";
  import RecentEvents from "./agent/RecentEvents.svelte";
  import ScheduleDetailModal from "./agent/ScheduleDetailModal.svelte";
  import ScheduleList from "./agent/ScheduleList.svelte";
  import { blankSchedule } from "./agent/blank";

  let data = $state<AgentView | null>(null);
  let error = $state("");
  let now = $state(new Date());
  let creating = $state(false);
  let editing = $state<Schedule | null>(null);
  let deleting = $state<Schedule | null>(null);
  let pausing = $state(false);

  async function load(): Promise<void> {
    try {
      const [local, remote] = await Promise.all([api.agent(), fromPeers<AgentView>("list_schedules")]);
      data = mergeAgent(local, remote);
      now = new Date();
      error = "";
      chatContext.text = JSON.stringify(data);
    } catch (e) {
      error = errorText(e);
    }
  }

  $effect(() => poll(load));

  async function setEnabled(s: ScheduleView, enabled: boolean): Promise<void> {
    try {
      await route.setScheduleEnabled(s.node, s.id, enabled);
    } catch (e) {
      error = errorText(e);
    }
    await load();
  }

  async function resume(): Promise<void> {
    try {
      await api.resumeAll();
    } catch (e) {
      error = errorText(e);
    }
    await load();
  }

  /** After Claude drafts a schedule, open it so the user can check it. */
  async function created(id: string): Promise<void> {
    creating = false;
    await load();
    editing = data?.schedules.find((s) => s.id === id) ?? null;
  }

  const editingView = $derived(data?.schedules.find((s) => s.id === editing?.id && s.node === editing?.node) ?? null);
</script>

<div class="column">
  <LoadError {error} />
  {#if data}
    <LiveRuns runs={data.live} turns={data.turns} />

    <section class="scheduled" aria-label={t("agent.scheduled")}>
      <div class="head">
        <h2>{t("agent.scheduled")}</h2>
        <div class="buttons">
          {#if !data.pause}
            <button type="button" class="button-secondary" onclick={() => (pausing = true)}>{t("agent.pauseAll")}</button>
          {/if}
          <button type="button" class="button-primary" onclick={() => (creating = true)}>{t("agent.new")}</button>
        </div>
      </div>
      {#if data.pause}<PausedBanner until={data.pause.until} onResume={resume} />{/if}
      {#if data.problems.length > 0}
        <div class="problems" role="alert">
          <p>{t("agent.problems")}</p>
          <ul>{#each data.problems as problem (problem)}<li class="mono">{problem}</li>{/each}</ul>
        </div>
      {/if}
      {#if data.schedules.filter(inArea).length === 0}<p class="hint">{t("empty.agent")}</p>{/if}
      <ScheduleList schedules={data.schedules.filter(inArea)} {now} paused={data.pause !== null} onOpen={(s) => (editing = s)} onToggle={setEnabled} onDelete={(s) => (deleting = s)} />
    </section>
    <RecentEvents events={data.events ?? []} schedules={data.schedules} {now} />
    <Connections connections={data.connections} {now} />
    <ConnectClaude />
  {/if}
</div>

{#if creating}
  <NewScheduleModal
    onClose={() => (creating = false)}
    onCreated={created}
    onManual={() => { creating = false; editing = blankSchedule(); }}
  />
{/if}

{#if pausing}
  <PauseModal onClose={() => (pausing = false)} onPaused={() => { pausing = false; void load(); }} />
{/if}

{#if deleting}
  <DeleteScheduleModal
    schedule={deleting}
    onClose={() => (deleting = null)}
    onDeleted={() => { deleting = null; void load(); }}
  />
{/if}

{#if editing}
  <ScheduleDetailModal
    schedule={editing}
    runs={editingView?.runs ?? []}
    needsApproval={editingView?.needsApproval ?? false}
    {now}
    onClose={() => (editing = null)}
    onSaved={() => { editing = null; void load(); }}
  />
{/if}

<style>
  .scheduled {
    margin-top: 44px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
  }

  .buttons {
    display: flex;
    gap: 8px;
  }

  .problems {
    margin-bottom: 12px;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--chip-decision-bg);
    color: var(--chip-decision-fg);
    font-size: 13px;
  }

  .problems li {
    margin-top: 4px;
    font-size: 12px;
  }
</style>
