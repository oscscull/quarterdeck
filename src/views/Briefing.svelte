<script lang="ts">
  import LoadError from "../components/LoadError.svelte";
  import { api, errorText } from "../lib/api";
  import { ALL, areas, findArea, inArea } from "../lib/areas.svelte";
  import { combined, isActive, mergeBriefing, projectKey, type Placed } from "../lib/briefing";
  import { chatContext } from "../lib/chat.svelte";
  import { t } from "../lib/copy";
  import { fromPeers, ownerOf } from "../lib/peers.svelte";
  import { poll } from "../lib/poll";
  import { dayClock } from "../lib/time";
  import type { BriefingView, ProjectView } from "../lib/types";
  import Bands from "./briefing/Bands.svelte";
  import Correction from "./briefing/Correction.svelte";
  import ProjectDetail from "./briefing/ProjectDetail.svelte";

  const AIMS = 3;

  let data = $state<BriefingView | null>(null);
  let error = $state("");
  let now = $state(new Date());
  /** The project shown, by `projectKey`; empty for everything. */
  let selected = $state("");

  async function load(): Promise<void> {
    try {
      const [local, remote] = await Promise.all([api.briefing(), fromPeers<BriefingView>("get_briefing")]);
      data = mergeBriefing(local, remote);
      now = new Date();
      error = "";
    } catch (e) {
      error = errorText(e);
    }
  }

  $effect(() => poll(load));

  const projects = $derived((data?.projects ?? []).filter(inArea).sort((a, b) => a.rank - b.rank));
  const active = $derived(projects.filter(isActive));
  const quiet = $derived(projects.length - active.length);
  const project = $derived(projects.find((p) => projectKey(p) === selected));
  const aims = $derived(active.filter((p) => p.aim).slice(0, AIMS));
  const bands = $derived(
    data ? combined(data, areas.list.map((a) => a.id), (area) => inArea({ area })) : { done: [], now: [], next: [] },
  );
  const written = $derived(
    (data?.areas ?? []).filter(inArea).map((a) => a.writtenAt).sort().at(-1),
  );
  const writing = $derived((data?.writing ?? []).some((area) => inArea({ area })));
  const hasText = $derived(bands.done.length + bands.now.length + bands.next.length > 0);

  function tagFor(p: Placed): { name: string; color: string; onOpen: () => void } | undefined {
    const found: ProjectView | undefined = data?.projects.find((x) => x.id === p.project && x.node === p.node);
    if (!found) return undefined;
    return {
      name: found.title,
      color: findArea(found.area)?.color ?? "var(--muted)",
      onOpen: () => (selected = projectKey(found)),
    };
  }

  const area = $derived(areas.current === ALL ? null : areas.current);
  const scope = $derived(
    project
      ? t("briefing.scope.project", { project: project.title })
      : area
        ? t("briefing.scope.area", { area: findArea(area)?.name ?? area })
        : t("briefing.scope.all"),
  );
  const owner = $derived(project ? project.node : area ? ownerOf(area) : undefined);

  $effect(() => {
    chatContext.text = JSON.stringify({ area: areas.current, project: project?.id ?? null, aims, bands });
  });
</script>

<div class="column">
  <LoadError {error} />
  {#if data}
    <nav class="pills" aria-label={t("view.briefing")}>
      <button type="button" class:on={!project} aria-current={!project ? "page" : undefined} onclick={() => (selected = "")}>
        {t("briefing.everything")}
      </button>
      {#each active as p (projectKey(p))}
        <button type="button" class:on={project === p} aria-current={project === p ? "page" : undefined} onclick={() => (selected = projectKey(p))}>
          <span class="dot" style:background={findArea(p.area)?.color ?? "var(--muted)"}></span>{p.title}
        </button>
      {/each}
      {#if quiet > 0}<span class="hint quiet">{t("briefing.quiet", { n: quiet })}</span>{/if}
    </nav>

    {#if project}
      <ProjectDetail {project} view={data} {now} />
    {:else}
      {#if aims.length > 0}
        <section class="aims" aria-labelledby="aims">
          <h2 id="aims" class="section-label">{t("briefing.aims")}</h2>
          <ul>
            {#each aims as p (projectKey(p))}
              <li><button type="button" onclick={() => (selected = projectKey(p))}>{p.aim}</button></li>
            {/each}
          </ul>
        </section>
      {/if}
      {#if hasText}
        <Bands {bands} view={data} {now} {tagFor} />
      {:else}
        <p class="hint">{t("briefing.empty")}</p>
      {/if}
    {/if}

    <p class="hint status">
      {#if writing}{t("briefing.writing")}{:else if written}{t("briefing.written", { when: dayClock(written, now) })}{/if}
      {#if !project && bands.done.length > 0}· {t("briefing.doneFades")}{/if}
    </p>

    {#key `${selected}|${areas.current}`}
      <Correction
        {scope}
        label={project ? t("briefing.correct.labelProject", { project: project.title }) : t("briefing.correct.label")}
        area={project?.area ?? area}
        project={project?.id ?? null}
        node={owner}
      />
    {/key}
  {/if}
</div>

<style>
  .pills {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-bottom: 36px;
  }

  .pills button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--surface);
    font-size: 13px;
    color: var(--ink);
  }

  .pills button.on {
    background: var(--ink);
    border-color: var(--ink);
    color: var(--surface);
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }

  .quiet {
    font-size: 13px;
  }

  .aims {
    margin-bottom: 36px;
  }

  .aims ul {
    margin-top: 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .aims button {
    padding: 0;
    border: 0;
    background: none;
    text-align: left;
    font-size: 22px;
    line-height: 1.3;
    font-weight: 600;
    letter-spacing: -0.01em;
    color: var(--ink);
  }

  .aims button:hover {
    color: var(--accent);
  }

  .status {
    margin-top: 8px;
    font-size: 12px;
  }
</style>
