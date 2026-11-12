<script lang="ts">
  import { api } from "../../lib/api";
  import { t } from "../../lib/copy";
  import { fromPeers, machine } from "../../lib/peers.svelte";
  import { runKey, runMeta, runName, stepText } from "../../lib/steps";
  import { age } from "../../lib/time";
  import type { AgentTurn, LiveRun } from "../../lib/types";
  import RunLogModal from "./RunLogModal.svelte";
  import WaitingTurns from "./WaitingTurns.svelte";

  interface Props {
    /** The Agent tab's last load, shown until this list's own first refresh. */
    runs: LiveRun[];
    /** Items that are the agent's turn, from the tab's last load. */
    turns: AgentTurn[];
  }

  let { runs, turns }: Props = $props();

  /** Much faster than the tab's 30s, so the latest step reads as live. */
  const REFRESH_MS = 3_000;

  let fresh = $state<LiveRun[] | null>(null);
  let now = $state(new Date());
  let openKey = $state<string | null>(null);
  /** The opened run as last seen, so its log stays up after it finishes. */
  let opened = $state<LiveRun | null>(null);

  const shown = $derived(fresh ?? runs);
  const openRun = $derived(shown.find((r) => runKey(r) === openKey));
  const waiting = $derived(
    turns.filter((a) => !shown.some((r) => r.target === a.itemId && (r.node ?? "") === (a.node ?? ""))),
  );
  const count = $derived(shown.length + waiting.length);

  async function load(): Promise<void> {
    try {
      const [local, remote] = await Promise.all([api.liveRuns(), fromPeers<LiveRun[]>("list_live_runs")]);
      const answered = new Set(remote.map((r) => r.node));
      // A peer without `list_live_runs` (an older build) still shows its runs from the tab's load.
      const unanswered = runs.filter((r) => r.node !== machine.name && !answered.has(r.node));
      fresh = [...local, ...remote.flatMap((r) => r.value), ...unanswered];
      now = new Date();
    } catch {
      // Keep the last list; the Agent tab reports load errors on its own refresh.
    }
  }

  $effect(() => {
    void load();
    const timer = setInterval(() => void load(), REFRESH_MS);
    return () => clearInterval(timer);
  });

  $effect(() => {
    if (openRun) opened = openRun;
  });

  function open(run: LiveRun): void {
    opened = run;
    openKey = runKey(run);
  }

  function close(): void {
    openKey = null;
    opened = null;
  }
</script>

<section aria-label={t("agent.live")}>
  <h1 class="page-title">
    {count === 0 ? t("agent.idle") : count === 1 ? t("agent.workingOne") : t("agent.working", { n: count })}
  </h1>
  <p class="hint intro">{t("agent.live")}</p>
  <ul>
    {#each shown as run (runKey(run))}
      <li>
        <button type="button" class="row-button" onclick={() => open(run)}>
          <span class="dot"></span>
          <span class="body">
            <span>{runName(run)}</span>
            <span class="hint meta">{runMeta(run)}</span>
            {#if run.now}<em class="now">{stepText(run.now)}</em>{/if}
          </span>
          <span class="mono elapsed">{age(run.startedAt, now)}</span>
        </button>
      </li>
    {/each}
    <WaitingTurns turns={waiting} {now} />
  </ul>
</section>

{#if opened}
  <RunLogModal run={opened} live={openRun !== undefined} onClose={close} />
{/if}

<style>
  .intro {
    margin: 6px 0 20px;
  }

  .row-button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 0;
    border: 0;
    background: transparent;
    text-align: left;
  }

  .dot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 50%;
    background: var(--live);
    box-shadow: 0 0 0 4px var(--live-halo);
  }

  .body {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .meta {
    font-size: 13px;
  }

  .now {
    font-size: 13px;
    color: var(--muted);
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }

  .elapsed {
    font-size: 13px;
    color: var(--muted);
  }
</style>
