<script lang="ts">
  import { chat, clearChat, sendChat } from "../lib/chat.svelte";
  import { t } from "../lib/copy";
  import { autogrow, enterSends } from "../lib/enter";

  interface Props {
    page: string;
    suggestions: string[];
  }

  let { page, suggestions }: Props = $props();

  const send = (): void => void sendChat(page);
</script>

<div class="dock">
  {#if chat.open}
    <section class:big={chat.big} aria-label={t("chat.title")}>
      <div class="head">
        <h2>{t("chat.title")}</h2>
        <div class="tools">
          {#if chat.messages.length > 0}
            <button type="button" class="clear" onclick={clearChat}>{t("chat.clear")}</button>
          {/if}
          <button
            type="button"
            class="icon"
            aria-label={t(chat.big ? "chat.smaller" : "chat.bigger")}
            title={t(chat.big ? "chat.smaller" : "chat.bigger")}
            onclick={() => (chat.big = !chat.big)}
          >
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              {#if chat.big}<path d="M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7"></path>{:else}<path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7"></path>{/if}
            </svg>
          </button>
          <button type="button" class="icon close" aria-label={t("chat.close")} onclick={() => (chat.open = false)}>×</button>
        </div>
      </div>
      <p class="hint context">{t("chat.looking", { page })}</p>
      {#if chat.messages.length === 0}
        <div class="suggestions">
          {#each suggestions as text (text)}
            <button type="button" onclick={() => (chat.draft = text)}>{text}</button>
          {/each}
        </div>
      {:else}
        <ol class="messages">
          {#each chat.messages as message, i (i)}
            <li class={message.from}>{message.text}</li>
          {/each}
        </ol>
      {/if}
      {#if chat.busy}<p class="hint context">{t("chat.thinking")}</p>{/if}
      {#if chat.error}<p class="error context">{chat.error}</p>{/if}
      <form onsubmit={(e) => { e.preventDefault(); send(); }}>
        <label class="grow">
          <span class="sr-only">{t("chat.placeholder")}</span>
          <textarea
            rows="1"
            bind:value={chat.draft}
            use:autogrow={chat.draft}
            onkeydown={(e) => enterSends(e, send)}
            placeholder={t("chat.placeholder")}
          ></textarea>
        </label>
        <button type="submit" class="send" aria-label={t("chat.send")} disabled={chat.busy}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 19V5M5 12l7-7 7 7"></path></svg>
        </button>
      </form>
    </section>
  {/if}
  <button type="button" class="bubble" aria-label={t("chat.open")} onclick={() => (chat.open = !chat.open)}>
    <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z"></path></svg>
  </button>
</div>

<style>
  .dock {
    position: fixed;
    right: 24px;
    bottom: 24px;
    z-index: 50;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 12px;
    font-size: 14px;
  }

  section {
    width: 360px;
    max-width: calc(100vw - 48px);
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 16px;
    box-shadow: 0 12px 40px rgba(25, 25, 23, 0.12);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  section.big {
    width: 720px;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 16px 10px;
  }

  h2 {
    font-size: 15px;
    font-weight: 600;
  }

  .tools {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .clear {
    padding: 6px 8px;
    border: 0;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
  }

  .icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    font-size: 18px;
  }

  .icon:hover {
    background: var(--surface-soft);
  }

  .context {
    margin: 0 16px 12px;
    font-size: 13px;
  }

  .suggestions {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 0 16px 14px;
  }

  .suggestions button {
    text-align: left;
    padding: 9px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--surface-soft);
    font-size: 13px;
  }

  .messages {
    max-height: 320px;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 0 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .big .messages {
    max-height: calc(100vh - 280px);
  }

  .messages li {
    max-width: 85%;
    overflow-wrap: anywhere;
    padding: 8px 12px;
    border-radius: 12px;
    line-height: 1.45;
    white-space: pre-wrap;
  }

  .you {
    align-self: flex-end;
    background: var(--ink);
    color: #fff;
  }

  .claude {
    align-self: flex-start;
    background: var(--surface-soft);
  }

  form {
    display: flex;
    gap: 8px;
    align-items: flex-end;
    padding: 12px 12px 12px 16px;
    border-top: 1px solid var(--line-soft);
  }

  .grow {
    flex: 1 1 0;
    min-width: 0;
  }

  textarea {
    display: block;
    width: 100%;
    max-height: 140px;
    padding: 8px 0;
    border: 0;
    outline: none;
    background: transparent;
    font: inherit;
    line-height: 1.45;
    overflow-wrap: anywhere;
    resize: none;
  }

  .big textarea {
    max-height: 280px;
  }

  .send,
  .bubble {
    border: 0;
    border-radius: 50%;
    background: var(--ink);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .send {
    width: 36px;
    height: 36px;
    flex: none;
  }

  .bubble {
    width: 52px;
    height: 52px;
    box-shadow: 0 6px 20px rgba(25, 25, 23, 0.16);
  }
</style>
