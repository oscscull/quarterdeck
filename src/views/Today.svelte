<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import AnswerBox from "../components/AnswerBox.svelte";
  import AreaDot from "../components/AreaDot.svelte";
  import TellClaude from "../components/TellClaude.svelte";
  import LoadError from "../components/LoadError.svelte";
  import { api, errorText } from "../lib/api";
  import { areas, inArea } from "../lib/areas.svelte";
  import { chatContext } from "../lib/chat.svelte";
  import { t } from "../lib/copy";
  import { poll } from "../lib/poll";
  import { clock, dayClock, until } from "../lib/time";
  import type { Commitment, TodayView } from "../lib/types";
  import { openItem } from "../lib/detail.svelte";
  import { parkedItems } from "../lib/items.svelte";
  import { mergeToday } from "../lib/merge";
  import { nav } from "../lib/nav.svelte";
  import { describe, keyOf } from "../lib/priority";
  import { fromPeers } from "../lib/peers.svelte";
  import { route } from "../lib/route";
  import { changed } from "../lib/refresh.svelte";
  import CommitmentModal from "./today/CommitmentModal.svelte";
  import TodoList from "./today/TodoList.svelte";
  import TriageLine from "./today/TriageLine.svelte";
  import ParkedLine from "./today/ParkedLine.svelte";
  import UrgentModal from "./today/UrgentModal.svelte";
  import UrgentPill from "./today/UrgentPill.svelte";

  let data = $state<TodayView | null>(null);
  let error = $state("");
  let now = $state(new Date());
  let showUrgent = $state(false);
  let addingCommitment = $state(false);

  async function hide(c: Commitment): Promise<void> {
    try {
      await route.hideMeeting(c.node, c.id);
      changed();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function load(): Promise<void> {
    try {
      const [local, remote] = await Promise.all([api.today(), fromPeers<TodayView>("get_overview")]);
      data = mergeToday(local, remote);
      now = new Date();
      error = "";
    } catch (e) {
      error = errorText(e);
    }
  }

  $effect(() => poll(load));

  const { state: parked, load: loadParked } = parkedItems();
  $effect(() => poll(loadParked));

  // Everything on Today follows the area switcher.
  const upcoming = $derived(data?.upcoming.filter(inArea) ?? []);
  const next = $derived(upcoming[0]);
  const then = $derived(upcoming[1]);
  const allPriorities = $derived(data?.priorities.filter(inArea) ?? []);
  const priorities = $derived(data ? allPriorities.slice(0, data.priorityLimit) : []);
  const hiddenPriorities = $derived(allPriorities.length - priorities.length);
  const todos = $derived(data?.todos.filter(inArea) ?? []);
  const triage = $derived(data?.triage?.filter(inArea) ?? []);
  const activity = $derived(data?.activity.filter(inArea) ?? []);
  const urgent = $derived(data?.urgent.filter(inArea) ?? []);

  $effect(() => {
    chatContext.text = JSON.stringify({ area: areas.current, upcoming, priorities, triage, todos, activity, urgent });
  });
</script>

<div class="column">
  <LoadError {error} />
  {#if data}
    <section class="next" aria-label={t("today.next")}>
      <div class="label-row">
        <span class="section-label">{t("today.next")}</span>
        <span class="label-actions">
          <UrgentPill count={urgent.length} onOpen={() => (showUrgent = true)} />
          <button type="button" class="link" onclick={() => (addingCommitment = true)}>{t("commitment.add")}</button>
        </span>
      </div>
      {#if next}
        <div class="title-row"><AreaDot area={next.area} /><h1>{next.title}</h1></div>
        <p class="when">
          <span class="mono">{clock(next.startsAt)}</span>
          <span class="muted">{t("today.in", { when: until(next.startsAt, now) })}</span>
          {#if next.location}<span class="muted">{t("today.where", { location: next.location })}</span>{/if}
          {#if next.joinUrl}{@const url = next.joinUrl}<button type="button" class="link" onclick={() => openUrl(url)}>{t("today.join")}</button>{/if}
          <button type="button" class="quiet" aria-label={t("today.hideHint", { title: next.title })} onclick={() => hide(next)}>{t("today.hide")}</button>
        </p>
        <p class="hint">
          {then ? t("today.then", { title: then.title, when: dayClock(then.startsAt, now) }) : t("today.nothingAfter")}
        </p>
      {:else}
        <p class="hint">{t("today.noCommitments")}</p>
      {/if}
    </section>

    <section aria-label={t("today.priorities")} class="block">
      <h2>{t("today.priorities")}</h2>
      <p class="hint sub">{t("today.prioritiesHint")}</p>
      <ul>
        {#each priorities as p (keyOf(p))}
          {@const line = describe(p)}
          <li class="row priority">
            <AreaDot area={p.area} />
            <button type="button" class="body open" onclick={() => openItem(p.itemId, p.node)}>
              <span>{line.text}</span>
              <span class="hint small">{line.context}</span>
            </button>
          </li>
          {#if p.kind === "ask"}
            <li class="answer-row"><AnswerBox ask={p.ask} node={p.node} /></li>
          {:else if p.kind === "yours"}
            <li class="answer-row"><TellClaude itemId={p.itemId} node={p.node} link={p.link} /></li>
          {/if}
        {:else}
          {#if triage.length === 0}
            <li class="hint">{t("today.noPriorities")}</li>
          {/if}
        {/each}
      </ul>
      {#if hiddenPriorities > 0}
        <button type="button" class="link more" onclick={() => (nav.view = "priorities")}>{t("today.more", { n: hiddenPriorities })}</button>
      {/if}
      <TriageLine issues={triage} />
      <ParkedLine items={parked.items.filter(inArea)} {now} />
    </section>

    <TodoList {todos} {now} onChanged={load} />

    <section aria-label={t("today.fromClaude")}>
      <h2 class="section-label">{t("today.fromClaude")}</h2>
      <ul class="activity">
        {#each activity as a (`${a.node ?? ""}${a.id}`)}
          <li class:unread={a.unread}>
            <span class="dot"></span>
            <AreaDot area={a.area} />
            <span class="text">{a.text}</span>
            <span class="mono time">{clock(a.at)}</span>
          </li>
        {/each}
      </ul>
      <p class="hint working">{t("today.working", { n: data.running })}</p>
    </section>
  {/if}
</div>

{#if addingCommitment}
  <CommitmentModal onClose={() => (addingCommitment = false)} />
{/if}

{#if showUrgent && data}
  <UrgentModal items={urgent} {now} onClose={() => (showUrgent = false)} onChanged={load} />
{/if}

<style>
  .more {
    margin-top: 8px;
  }

  .label-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .link {
    padding: 8px 0;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 13px;
  }

  .open {
    padding: 0;
    border: 0;
    background: transparent;
    text-align: left;
  }

  .quiet {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--faint);
    font-size: 13px;
  }

  .answer-row {
    padding: 0 0 14px 22px;
  }

  .label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    min-height: 36px;
  }

  .next {
    padding: 24px 0 48px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  h1 {
    font-size: 44px;
    font-weight: 600;
    letter-spacing: -0.025em;
    line-height: 1.05;
  }

  .when {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 18px;
  }

  .when .link {
    padding: 0;
    font-size: inherit;
  }

  .muted {
    color: var(--muted);
  }

  .block {
    padding-bottom: 48px;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
  }

  .sub {
    margin: 4px 0 16px;
    font-size: 13px;
  }

  .priority {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 14px 0;
  }

  .priority .body {
    flex: 1 1 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .small {
    font-size: 13px;
  }

  .activity {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .activity li {
    display: flex;
    align-items: baseline;
    gap: 10px;
    padding: 6px 0;
    font-size: 14px;
    color: var(--muted);
  }

  .activity .unread {
    color: var(--ink);
  }

  .dot {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
    transform: translateY(-2px);
  }

  .unread .dot {
    background: var(--accent);
  }

  .text {
    flex: 1 1 0;
    min-width: 0;
  }

  .time {
    font-size: 12px;
    color: var(--faint);
  }

  .working {
    margin: 12px 0 0 16px;
    font-size: 13px;
  }
</style>
