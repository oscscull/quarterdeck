<script lang="ts">
  import { placed, type Band, type Placed } from "../../lib/briefing";
  import { t } from "../../lib/copy";
  import { dayClock } from "../../lib/time";
  import type { BriefingView, ProjectView } from "../../lib/types";
  import Bands from "./Bands.svelte";
  import ProjectItems from "./ProjectItems.svelte";

  interface Props {
    project: ProjectView;
    view: BriefingView;
    now: Date;
  }

  let { project, view, now }: Props = $props();

  const bands: Record<Band, Placed[]> = $derived(placed(project.briefing, project.node));
  const memberIds = $derived(new Set(project.items.map((i) => i.id)));
  const sources = $derived(
    view.references.filter(
      (r) => r.node === project.node && (r.project === project.id || r.items.some((i) => memberIds.has(i))),
    ),
  );
</script>

<p class="meta">
  <span class="state" class:stalled={project.state === "stalled"}>{t(`briefing.state.${project.state}`)}</span>
  {t("briefing.since", { when: dayClock(project.stateSince, now) })}
</p>
<h1 class="aim">{project.aim || project.title}</h1>

{#if project.briefing}
  <Bands {bands} {view} {now} />
{:else}
  <p class="hint empty">{t("briefing.empty")}</p>
{/if}

<ProjectItems items={project.items} />

<details>
  <summary class="section-label">{t("briefing.references", { n: sources.length })}</summary>
  <ol class="refs">
    {#each sources as r (r.id)}
      <li><span class="title">{r.title}</span> <span class="hint">{r.quote}</span></li>
    {/each}
  </ol>
</details>

<style>
  .meta {
    margin-bottom: 6px;
    font-size: 13px;
    color: var(--muted);
  }

  .state {
    font-weight: 600;
    color: var(--ink);
  }

  .state.stalled {
    color: var(--chip-decision-fg);
  }

  .aim {
    margin-bottom: 32px;
    font-size: 26px;
    line-height: 1.3;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .empty {
    margin-bottom: 30px;
  }

  details {
    margin-top: 28px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }

  summary {
    cursor: pointer;
  }

  .refs {
    margin-top: 8px;
  }

  .refs li {
    padding: 8px 0;
    border-top: 1px solid var(--line-soft);
    font-size: 14px;
  }

  .title {
    font-weight: 500;
  }
</style>
