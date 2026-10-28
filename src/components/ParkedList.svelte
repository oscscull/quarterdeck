<script lang="ts">
  import AreaDot from "./AreaDot.svelte";
  import { t } from "../lib/copy";
  import { openItem } from "../lib/detail.svelte";
  import { dateClock, until } from "../lib/time";
  import type { WorkItem } from "../lib/types";

  interface Props {
    /** Soonest back first. */
    items: WorkItem[];
    now: Date;
  }

  let { items, now }: Props = $props();

  const who = (item: WorkItem): string => t(item.thenTurn === "agent" ? "parked.thenAgent" : "parked.thenYou");
</script>

<ul class="list">
  {#each items as item (`${item.node ?? ""}|${item.id}`)}
    <li>
      <AreaDot area={item.area} />
      <span class="body">
        <button type="button" class="open" onclick={() => openItem(item.id, item.node)}>
          <span class="mono id">{item.id}</span>&nbsp; {item.title}
        </button>
        {#if item.need}<span class="hint note">{item.need}</span>{/if}
        <span class="hint meta">
          {item.turnUntil ? t("parked.backOn", { at: dateClock(item.turnUntil), who: who(item) }) : t("parked.onEvent", { who: who(item) })}
        </span>
      </span>
      {#if item.turnUntil}
        <span class="mono countdown">{until(item.turnUntil, now)}</span>
      {/if}
    </li>
  {:else}
    <li class="hint empty">{t("parked.empty")}</li>
  {/each}
</ul>

<style>
  .list {
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--surface);
    overflow: hidden;
  }

  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--line-soft);
  }

  li:first-child {
    border-top: 0;
  }

  .body {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .open {
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
  }

  .id {
    font-size: 12px;
    color: var(--muted);
  }

  .note,
  .meta {
    font-size: 13px;
  }

  .countdown {
    flex: none;
    font-size: 14px;
    color: var(--accent);
  }

  .empty {
    padding: 20px 16px;
  }
</style>
