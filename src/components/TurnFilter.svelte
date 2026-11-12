<script lang="ts">
  import { t } from "../lib/copy";
  import { TURN_FILTERS, matchesTurn, type TurnFilter } from "../lib/groups";
  import type { WorkItem } from "../lib/types";

  interface Props {
    items: WorkItem[];
    value: TurnFilter;
    onSelect: (value: TurnFilter) => void;
  }

  let { items, value, onSelect }: Props = $props();

  const options = $derived(
    TURN_FILTERS.map((f) => ({ ...f, n: items.filter((i) => matchesTurn(i, f.id)).length })).filter(
      (f) => f.id === "all" || f.id === value || f.n > 0,
    ),
  );
</script>

<nav aria-label={t("priorities.filter.label")}>
  {#each options as o (o.id)}
    <button type="button" class:active={value === o.id} aria-pressed={value === o.id} onclick={() => onSelect(o.id)}>
      {t(o.label)} <span class="mono count">{o.n}</span>
    </button>
  {/each}
</nav>

<style>
  nav {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    margin-bottom: 12px;
  }

  button {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 7px 10px;
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
  }

  .active {
    background: var(--surface);
    color: var(--ink);
    font-weight: 500;
  }

  .count {
    font-size: 12px;
    color: var(--faint);
  }
</style>
