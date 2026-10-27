<script lang="ts">
  import { areas } from "../../lib/areas.svelte";
  import { t } from "../../lib/copy";

  export interface TodoFields {
    title: string;
    area: string;
    due: string;
    link: string;
  }

  interface Props {
    start: TodoFields;
    /** Shown on the save button: "Add" for new to-dos, "Save" for edits. */
    saveLabel: string;
    onSave: (fields: TodoFields) => void;
    onCancel: () => void;
    /** Only when editing. */
    onDelete?: () => void;
  }

  let { start, saveLabel, onSave, onCancel, onDelete }: Props = $props();

  // A fresh copy to edit; the form is re-created for each to-do it opens.
  // svelte-ignore state_referenced_locally
  let fields = $state({ ...start });
</script>

<form onsubmit={(e) => { e.preventDefault(); if (fields.title.trim()) onSave({ ...fields, title: fields.title.trim() }); }}>
  <label class="field what">
    {t("todo.what")}
    <!-- svelte-ignore a11y_autofocus -->
    <input class="input" bind:value={fields.title} placeholder={t("todo.placeholder")} autofocus />
  </label>
  <label class="field">
    {t("todo.area")}
    <select class="input" bind:value={fields.area}>
      {#each areas.list as a (a.id)}<option value={a.id}>{a.name}</option>{/each}
    </select>
  </label>
  <label class="field">
    {t("todo.due")}
    <input class="input" type="date" bind:value={fields.due} />
  </label>
  <label class="field what">
    {t("todo.link")}
    <input class="input" type="url" bind:value={fields.link} placeholder={t("todo.linkPlaceholder")} />
  </label>
  <div class="buttons">
    {#if onDelete}
      <button type="button" class="delete" onclick={onDelete}>{t("todo.delete")}</button>
    {/if}
    <button type="button" class="button-secondary" onclick={onCancel}>{t("todo.cancel")}</button>
    <button type="submit" class="button-primary" disabled={!fields.title.trim()}>{saveLabel}</button>
  </div>
</form>

<style>
  form {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px 12px;
    margin: 6px 0 16px;
    padding: 14px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--line);
  }

  .what {
    grid-column: 1 / -1;
  }

  .buttons {
    grid-column: 1 / -1;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .delete {
    margin-right: auto;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
  }

  .delete:hover {
    color: var(--urgent-fg);
  }
</style>
