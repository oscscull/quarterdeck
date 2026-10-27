<script lang="ts">
  import ChatBubble from "./components/ChatBubble.svelte";
  import Header from "./components/Header.svelte";
  import ItemDetailModal from "./components/item/ItemDetailModal.svelte";
  import ItemEditor from "./components/item/ItemEditor.svelte";
  import { detail, openItem } from "./lib/detail.svelte";
  import { loadAreas } from "./lib/areas.svelte";
  import { nav } from "./lib/nav.svelte";
  import { loadPeers } from "./lib/peers.svelte";
  import { reloadOnReturn } from "./lib/refresh.svelte";
  import { t } from "./lib/copy";
  import { showVersion } from "./lib/version";
  import { suggestionKeys, viewLabel } from "./lib/views";
  import Agent from "./views/Agent.svelte";
  import Board from "./views/Board.svelte";
  import InFlight from "./views/InFlight.svelte";
  import Priorities from "./views/Priorities.svelte";
  import Today from "./views/Today.svelte";

  const view = $derived(nav.view);

  const PEERS_EVERY_MS = 60_000;

  $effect(() => {
    void loadAreas();
    void loadPeers();
    const timer = setInterval(() => void loadPeers(), PEERS_EVERY_MS);
    return () => clearInterval(timer);
  });

  $effect(() => reloadOnReturn());
  $effect(() => void showVersion());
</script>

<Header {view} onSelect={(next) => (nav.view = next)} />

{#if view === "today"}
  <Today />
{:else if view === "priorities"}
  <Priorities />
{:else if view === "board"}
  <Board />
{:else if view === "inflight"}
  <InFlight />
{:else}
  <Agent />
{/if}

<ChatBubble page={t(viewLabel(view))} suggestions={suggestionKeys(view).map((key) => t(key))} />

{#if detail.id}
  <ItemDetailModal id={detail.id} node={detail.node} onClose={() => (detail.id = null)} />
{/if}

{#if detail.editing}
  <ItemEditor
    item={detail.editing}
    onClose={() => (detail.editing = null)}
    onSaved={(id, node) => { detail.editing = null; openItem(id, node); }}
  />
{/if}
