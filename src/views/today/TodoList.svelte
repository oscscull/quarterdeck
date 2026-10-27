<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";

  import AreaDot from "../../components/AreaDot.svelte";
  import { errorText } from "../../lib/api";
  import { ownerOf } from "../../lib/peers.svelte";
  import { route } from "../../lib/route";
  import { ALL, areas } from "../../lib/areas.svelte";
  import { t } from "../../lib/copy";
  import { dueText, isOverdue } from "../../lib/time";
  import type { Todo } from "../../lib/types";
  import TodoForm, { type TodoFields } from "./TodoForm.svelte";

  interface Props {
    todos: Todo[];
    now: Date;
    onChanged: () => void;
  }

  let { todos, now, onChanged }: Props = $props();

  let adding = $state(false);
  /** The to-do open for editing, by its list key. */
  let editing = $state<string | null>(null);
  let error = $state("");

  const keyOf = (todo: Todo): string => `${todo.node ?? ""}${todo.id}`;

  function startAdding(): void {
    editing = null;
    adding = true;
  }

  function startEditing(todo: Todo): void {
    adding = false;
    editing = keyOf(todo);
  }

  const newFields = (): TodoFields => ({
    title: "",
    area: areas.current !== ALL ? areas.current : (areas.list[0]?.id ?? ""),
    due: "",
    link: "",
  });

  async function run(work: () => Promise<unknown>): Promise<void> {
    try {
      await work();
      adding = false;
      editing = null;
      error = "";
      onChanged();
    } catch (e) {
      error = t("todo.failed", { error: errorText(e) });
    }
  }

  const add = (f: TodoFields): Promise<void> =>
    run(() => route.addTodo(ownerOf(f.area), f.title, f.area, f.due || null, f.link.trim() || null));

  const save = (todo: Todo, f: TodoFields): Promise<void> =>
    run(() => route.updateTodo(todo.node, { id: todo.id, title: f.title, area: f.area, due: f.due || null, link: f.link.trim() || null }));

  const toggle = (todo: Todo): Promise<void> => run(() => route.setTodoDone(todo.node, todo.id, todo.doneAt === null));

  const late = (todo: Todo): boolean => todo.doneAt === null && todo.due !== null && isOverdue(todo.due, now);
</script>

<section aria-label={t("todo.title")}>
  <div class="head">
    <h2>{t("todo.title")}</h2>
    {#if !adding}<button type="button" class="link" onclick={startAdding}>{t("todo.add")}</button>{/if}
  </div>
  <p class="hint sub">{t("todo.hint")}</p>

  {#if adding}
    <TodoForm start={newFields()} saveLabel={t("todo.save")} onSave={add} onCancel={() => (adding = false)} />
  {/if}
  {#if error}<p class="error">{error}</p>{/if}

  <ul>
    {#each todos as todo (keyOf(todo))}
      {#if editing === keyOf(todo)}
        <li>
          <TodoForm
            start={{ title: todo.title, area: todo.area, due: todo.due ?? "", link: todo.link ?? "" }}
            saveLabel={t("todo.update")}
            onSave={(f) => save(todo, f)}
            onCancel={() => (editing = null)}
            onDelete={() => run(() => route.deleteTodo(todo.node, todo.id))}
          />
        </li>
      {:else}
        <li class="row" class:done={todo.doneAt !== null}>
          <input
            type="checkbox"
            class="check"
            checked={todo.doneAt !== null}
            onchange={() => toggle(todo)}
            aria-label={todo.title}
          />
          <button type="button" class="open" onclick={() => startEditing(todo)} aria-label={t("todo.edit", { title: todo.title })}>
            <span class="title">{todo.title}</span>
            <AreaDot area={todo.area} />
            <span class="due" class:late={late(todo)}>{todo.due ? dueText(todo.due, now) : ""}</span>
          </button>
          {#if todo.link}
            {@const link = todo.link}
            <button type="button" class="visit" onclick={() => openUrl(link)} aria-label={t("todo.openLink", { title: todo.title })} title={link}>↗</button>
          {/if}
        </li>
      {/if}
    {:else}
      <li class="row hint empty">{t("todo.empty")}</li>
    {/each}
  </ul>
</section>

<style>
  section {
    padding-bottom: 44px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
  }

  .sub {
    margin: 4px 0 12px;
    font-size: 13px;
  }

  .link {
    padding: 8px 0;
    border: 0;
    background: transparent;
    color: var(--accent);
    font-size: 13px;
  }


  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
  }

  .check {
    width: 18px;
    height: 18px;
    flex: none;
    margin: 7px 0;
    accent-color: var(--ink);
    cursor: pointer;
  }

  .open {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 32px;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .open .title {
    flex: 1 1 0;
    min-width: 0;
  }

  .open:hover .title {
    text-decoration: underline;
    text-decoration-color: var(--line);
    text-underline-offset: 3px;
  }

  .visit {
    flex: none;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--accent);
    font-size: 15px;
  }

  .visit:hover {
    background: var(--surface-soft);
  }

  .done {
    opacity: 0.5;
  }

  .done .title,
  .done .open:hover .title {
    text-decoration: line-through;
  }

  .due {
    width: 84px;
    text-align: right;
    font-size: 13px;
    color: var(--muted);
  }

  .late {
    color: var(--urgent-fg);
  }

  .empty {
    font-size: 14px;
  }
</style>
