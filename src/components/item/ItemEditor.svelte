<script lang="ts">
  import { untrack } from "svelte";

  import { errorText } from "../../lib/api";
  import { areas } from "../../lib/areas.svelte";
  import { t, type CopyKey } from "../../lib/copy";
  import { STAGES } from "../../lib/groups";
  import { changed } from "../../lib/refresh.svelte";
  import type { Editing } from "../../lib/detail.svelte";
  import { ownerOf } from "../../lib/peers.svelte";
  import { route } from "../../lib/route";
  import type { ItemInput, Stage, Status } from "../../lib/types";
  import Modal from "../Modal.svelte";

  interface Props {
    item: Editing;
    onClose: () => void;
    onSaved: (id: string, node: string | undefined) => void;
  }

  let { item, onClose, onSaved }: Props = $props();

  const STATUSES: Status[] = ["approved", "review", "ci", "ci_failed", "dependency", "queued"];
  const stageKey = (s: Stage): CopyKey => `stage.${s}`;
  const statusKey = (s: Status): CopyKey => `status.${s}`;

  // Edit a copy; nothing is saved until Save.
  const start = untrack(() => item);
  let form = $state<ItemInput>({ ...start });
  let link = $state(untrack(() => item.link) ?? "");
  let error = $state("");
  const isNew = start.id === "";

  async function save(): Promise<void> {
    try {
      // Existing items stay where they are; new ones go to the machine that runs their area.
      const node = isNew ? ownerOf(form.area) : start.node;
      const id = await route.saveItem(node, { ...$state.snapshot(form), link: link.trim() || null });
      changed();
      onSaved(id, node);
    } catch (e) {
      error = t("item.failed", { error: errorText(e) });
    }
  }
</script>

<Modal title={isNew ? t("item.newTitle") : t("item.editTitle", { id: form.id })} {onClose}>
  <form class="grid" onsubmit={(e) => { e.preventDefault(); void save(); }}>
    {#if isNew}
      <label class="field">
        {t("item.key")}
        <input class="input mono" bind:value={form.id} placeholder={t("item.keyPlaceholder")} />
      </label>
    {/if}
    <label class="field" class:wide={!isNew}>
      {t("item.area")}
      <select class="input" bind:value={form.area}>
        {#each areas.list as a (a.id)}<option value={a.id}>{a.name}</option>{/each}
      </select>
    </label>
    <label class="field wide">
      {t("item.title")}
      <input class="input" bind:value={form.title} required />
    </label>
    <label class="field">
      {t("item.stage")}
      <select class="input" bind:value={form.stage}>
        {#each STAGES as s (s)}<option value={s}>{t(stageKey(s))}</option>{/each}
      </select>
    </label>
    <label class="field">
      {t("item.status")}
      <select class="input" bind:value={form.status}>
        <option value={null}>{t("item.noStatus")}</option>
        {#each STATUSES as s (s)}<option value={s}>{t(statusKey(s))}</option>{/each}
      </select>
    </label>
    <label class="field wide">
      {t("item.label")}
      <input class="input" bind:value={form.stateLabel} placeholder={t("item.labelPlaceholder")} />
    </label>
    <label class="field">
      {t("item.link")}
      <input class="input" type="url" bind:value={link} placeholder="https://" />
    </label>
    {#if error}<p class="error wide">{error}</p>{/if}
    <div class="buttons wide">
      <button type="button" class="button-secondary" onclick={onClose}>{t("item.cancel")}</button>
      <button type="submit" class="button-primary" disabled={!form.title.trim()}>{t("item.save")}</button>
    </div>
  </form>
</Modal>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .wide {
    grid-column: 1 / -1;
  }

  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
