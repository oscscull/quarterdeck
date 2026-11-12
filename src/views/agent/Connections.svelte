<script lang="ts">
  import { t, type CopyKey } from "../../lib/copy";
  import { machine } from "../../lib/peers.svelte";
  import { age } from "../../lib/time";
  import type { Connection } from "../../lib/types";
  import Calendars from "./Calendars.svelte";

  interface Props {
    connections: Connection[];
    now: Date;
  }

  let { connections, now }: Props = $props();

  const NAMES: Record<string, CopyKey> = {
    github: "connections.github",
    "github-issue": "connections.githubIssues",
    calendar: "connections.calendar",
  };
  const nameOf = (c: Connection): string => {
    const key = NAMES[c.name];
    return key ? t(key) : c.name;
  };

  function stateOf(c: Connection): string {
    if (c.error) return t("connections.failed", { error: c.error });
    return c.lastSync ? t("connections.synced", { when: age(c.lastSync, now) }) : t("connections.never");
  }
</script>

<section aria-label={t("connections.title")}>
  <h2 class="section-label">{t("connections.title")}</h2>
  <ul>
    {#each connections as c (c.name)}
      {#if c.name === "calendar"}
        <Calendars syncText={stateOf(c)} failed={Boolean(c.error)} />
      {:else}
        <li><span class="name">{nameOf(c)}</span> <span class:error={c.error} class="hint">{stateOf(c)}</span></li>
      {/if}
    {/each}
    {#each machine.peers as p (p.name)}
      <li>
        <span class="name">{p.name}</span>
        <span class:error={p.error} class="hint">{p.error ? t("connections.peerDown", { error: p.error }) : t("connections.peerUp")}</span>
      </li>
    {/each}
  </ul>
</section>

<style>
  section {
    margin-top: 44px;
  }

  ul {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 14px;
  }

  .name {
    font-weight: 500;
  }

  .hint {
    font-size: 13px;
  }

  .error {
    color: var(--urgent-fg);
  }
</style>
