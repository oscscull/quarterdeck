<script lang="ts">
  import { t } from "../../lib/copy";
  import { nav } from "../../lib/nav.svelte";
  import { until } from "../../lib/time";
  import type { WorkItem } from "../../lib/types";

  interface Props {
    /** Soonest back first. */
    items: WorkItem[];
    now: Date;
  }

  let { items, now }: Props = $props();

  const next = $derived(items.find((i) => i.turnUntil)?.turnUntil);
  const count = $derived(items.length === 1 ? t("today.parkedOne") : t("today.parked", { n: items.length }));
</script>

{#if items.length > 0}
  <button type="button" class="link" onclick={() => (nav.view = "priorities")}>
    {next ? t("today.parkedNext", { count, at: until(next, now) }) : count}
  </button>
{/if}

<style>
  .link {
    display: block;
    padding: 8px 0;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 13px;
  }
</style>
