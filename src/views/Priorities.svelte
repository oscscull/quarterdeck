<script lang="ts">
  import Age from "../components/Age.svelte";
  import AreaDot from "../components/AreaDot.svelte";
  import Chip from "../components/Chip.svelte";
  import LoadError from "../components/LoadError.svelte";
  import ParkedList from "../components/ParkedList.svelte";
  import TurnFilter from "../components/TurnFilter.svelte";
  import { api, errorText } from "../lib/api";
  import { t } from "../lib/copy";
  import { isInFlight, matchesTurn, type TurnFilter as Filter } from "../lib/groups";
  import { age } from "../lib/time";
  import { turnText } from "../lib/turn";
  import { ALL, areas, inArea } from "../lib/areas.svelte";
  import { newItem, openItem } from "../lib/detail.svelte";
  import { chatContext } from "../lib/chat.svelte";
  import { parkedItems, workItems } from "../lib/items.svelte";
  import { poll } from "../lib/poll";
  import { reorder } from "../lib/rank";
  import { route } from "../lib/route";
  import type { WorkItem } from "../lib/types";

  const { state: list, load } = workItems();
  let dragging = $state<string | null>(null);
  let over = $state<number | null>(null);
  let saveError = $state("");
  let filter = $state<Filter>("all");

  $effect(() => poll(load));

  const { state: parked, load: loadParked } = parkedItems();
  $effect(() => poll(loadParked));
  const parkedHere = $derived(parked.items.filter(inArea));

  const inPlay = $derived(list.items.filter((i) => i.stage !== "done" && inArea(i)));
  const visible = $derived(inPlay.filter((i) => matchesTurn(i, filter)));
  const oldest = $derived(
    inPlay.filter(isInFlight).reduce<string | null>((min, i) => (min === null || i.openedAt < min ? i.openedAt : min), null),
  );

  $effect(() => {
    chatContext.text = JSON.stringify({ area: areas.current, turn: filter, ranked: visible, parked: parkedHere });
  });

  // Ids repeat across machines, so the list is keyed by machine and id.
  const keyOf = (i: WorkItem): string => `${i.node ?? ""}|${i.id}`;

  /** Saves the order on the machine that owns `moved`. Each machine ranks its own items. */
  async function save(moved: WorkItem): Promise<void> {
    const same = list.items.filter((i) => i.node === moved.node);
    if (moved.node) {
      await route.moveItem(moved.node, moved.id, same.findIndex((i) => i.id === moved.id) + 1);
    } else {
      await api.reorderItems(same.map((i) => i.id));
    }
  }

  /** Moves `key` to `to` within the visible list. */
  async function move(key: string, to: number): Promise<void> {
    const keys = reorder(list.items.map(keyOf), visible.map(keyOf), key, to);
    const byKey = new Map(list.items.map((i) => [keyOf(i), i]));
    list.items = keys.flatMap((k) => byKey.get(k) ?? []);
    const moved = byKey.get(key);
    if (!moved) return;
    try {
      await save(moved);
      saveError = "";
    } catch (e) {
      saveError = t("priorities.saveFailed", { error: errorText(e) });
    }
  }

  function drop(index: number): void {
    const key = dragging;
    dragging = null;
    over = null;
    if (!key) return;
    const from = visible.findIndex((i) => keyOf(i) === key);
    void move(key, from < index ? index - 1 : index);
  }

  const meta = (item: WorkItem): string =>
    [t(`stage.${item.stage}`), turnText(item), item.stateLabel].filter(Boolean).join(" · ");
</script>

<div class="column">
  <div class="title-row">
    <h1 class="page-title">{t("priorities.title")}</h1>
    <button type="button" class="button-primary" onclick={() => newItem(areas.current !== ALL ? areas.current : (areas.list[0]?.id ?? ""))}>{t("item.new")}</button>
  </div>
  <p class="hint intro">
    {t("priorities.hint")}
    {#if oldest}{t("priorities.oldest", { age: age(oldest, list.now) })}{/if}
  </p>
  <LoadError error={list.error} />
  <TurnFilter items={inPlay} value={filter} onSelect={(f) => (filter = f)} />
  {#if saveError}<p class="error">{saveError}</p>{/if}

  <ol class="list">
    {#each visible as item, i (keyOf(item))}
      <li
        draggable="true"
        class:dragging={dragging === keyOf(item)}
        class:over={over === i && dragging !== keyOf(item)}
        ondragstart={(e) => { e.dataTransfer?.setData("text/plain", keyOf(item)); dragging = keyOf(item); }}
        ondragover={(e) => { e.preventDefault(); over = i; }}
        ondrop={(e) => { e.preventDefault(); drop(i); }}
        ondragend={() => { dragging = null; over = null; }}
      >
        <svg class="grip" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><circle cx="9" cy="6" r="1.6"></circle><circle cx="15" cy="6" r="1.6"></circle><circle cx="9" cy="12" r="1.6"></circle><circle cx="15" cy="12" r="1.6"></circle><circle cx="9" cy="18" r="1.6"></circle><circle cx="15" cy="18" r="1.6"></circle></svg>
        <span class="mono rank">{inPlay.indexOf(item) + 1}</span>
        <AreaDot area={item.area} />
        <span class="body">
          <button type="button" class="open" onclick={() => openItem(item.id, item.node)}><span class="mono id">{item.id}</span>&nbsp; {item.title}</button>
          <span class="hint meta">{meta(item)}</span>
        </span>
        {#if item.status}<Chip status={item.status} />{/if}
        {#if isInFlight(item)}<span class="age"><Age since={item.openedAt} now={list.now} /></span>{/if}
        <span class="arrows">
          <button type="button" aria-label={t("priorities.moveUp", { title: item.title })} onclick={() => move(keyOf(item), i - 1)}>▲</button>
          <button type="button" aria-label={t("priorities.moveDown", { title: item.title })} onclick={() => move(keyOf(item), i + 1)}>▼</button>
        </span>
      </li>
    {:else}
      <li class="hint empty">{t(filter === "all" ? "empty.priorities" : "empty.prioritiesTurn")}</li>
    {/each}
  </ol>

  <section class="parked" aria-label={t("parked.title")}>
    <h2 class="section-label">{t("parked.title")} <span class="mono count">{parkedHere.length}</span></h2>
    <p class="hint intro-small">{t("parked.hint")}</p>
    <LoadError error={parked.error} />
    <ParkedList items={parkedHere} now={parked.now} />
  </section>
</div>

<style>
  .title-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }

  .open {
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
  }

  .parked {
    margin-top: 40px;
  }

  .intro-small {
    margin: 4px 0 12px;
    font-size: 13px;
  }

  .count {
    color: var(--muted);
  }

  .empty {
    cursor: default;
    padding: 20px 16px;
  }

  .intro {
    margin: 6px 0 28px;
  }

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
    padding: 12px 12px 12px 8px;
    border-top: 1px solid var(--line-soft);
    cursor: grab;
  }

  li:first-child {
    border-top: 0;
  }

  .dragging {
    background: #f2f0eb;
  }

  .over {
    box-shadow: inset 0 2px 0 var(--accent);
  }

  .grip {
    flex: none;
    color: #9c9991;
  }

  .rank {
    width: 20px;
    flex: none;
    text-align: right;
    font-size: 13px;
    color: var(--muted);
  }

  .body {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .id {
    font-size: 12px;
    color: var(--muted);
  }

  .meta {
    font-size: 13px;
  }

  .age {
    flex: none;
    width: 32px;
    text-align: right;
  }

  .arrows {
    display: flex;
    flex-direction: column;
    flex: none;
  }

  .arrows button {
    width: 44px;
    height: 22px;
    border: 0;
    background: transparent;
    color: var(--muted);
    line-height: 1;
  }
</style>
