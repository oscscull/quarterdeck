<script lang="ts">
  import Modal from "../../components/Modal.svelte";
  import { errorText } from "../../lib/api";
  import { t } from "../../lib/copy";
  import { machine } from "../../lib/peers.svelte";
  import { route } from "../../lib/route";
  import { runMeta, runName, stepText } from "../../lib/steps";
  import { clock } from "../../lib/time";
  import type { LiveRun, Step } from "../../lib/types";

  interface Props {
    run: LiveRun;
    /** False once the run has finished: its log is gone, so the last one fetched stays up. */
    live: boolean;
    onClose: () => void;
  }

  let { run, live, onClose }: Props = $props();

  const REFRESH_MS = 2_000;

  let steps = $state<Step[]>([]);
  let error = $state("");

  const node = $derived(run.node === machine.name ? undefined : run.node);

  async function load(): Promise<void> {
    try {
      const latest = await route.runLog(node, run.id);
      // A finished run's log is deleted; keep what was already shown.
      if (latest.length > 0 || steps.length === 0) steps = latest;
      error = "";
    } catch (e) {
      error = t("runLog.failed", { error: errorText(e) });
    }
  }

  $effect(() => {
    if (!live) return;
    void load();
    const timer = setInterval(() => void load(), REFRESH_MS);
    return () => clearInterval(timer);
  });
</script>

<Modal title={runName(run)} subtitle={runMeta(run)} {onClose}>
  {#if error}<p class="error">{error}</p>{/if}
  {#if steps.length === 0 && live}<p class="hint">{t("runLog.empty")}</p>{/if}
  <ol>
    {#each steps as step, i (i)}
      <li class:failed={step.kind === "failed"} class:said={step.kind === "said"} class:output={step.kind === "output"}>
        <span class="mono at">{clock(step.at)}</span>
        <span class="text">{stepText(step)}</span>
      </li>
    {/each}
  </ol>
  {#if !live}<p class="hint">{t("runLog.finished")}</p>{/if}
</Modal>

<style>
  ol {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 60vh;
    overflow-y: auto;
  }

  li {
    display: flex;
    gap: 12px;
    font-size: 14px;
    color: var(--muted);
  }

  li.output .text {
    font-family: var(--mono, ui-monospace, monospace);
    font-size: 13px;
  }

  li.said {
    color: inherit;
  }

  li.failed {
    color: var(--chip-decision-fg);
  }

  .at {
    flex: none;
    font-size: 12px;
    padding-top: 2px;
  }

  .text {
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>
