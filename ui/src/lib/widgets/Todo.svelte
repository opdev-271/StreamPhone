<script lang="ts">
  import { Check, Plus, X } from '@lucide/svelte';
  import { buzz, conn } from '../conn.svelte';
  import type { TodoItem, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  $effect(() => conn.subscribe('todo.list'));

  const items = $derived((conn.data['todo.list'] as TodoItem[] | undefined) ?? []);
  const open = $derived(items.filter((t) => !t.done).length);
  const title = $derived((def.config?.title as string) || 'Úkoly');

  let draft = $state('');

  function add(e: Event) {
    e.preventDefault();
    const text = draft.trim();
    if (!text) return;
    buzz();
    conn.action({ do: 'todo.add', text });
    draft = '';
  }

  function toggle(id: number) {
    buzz();
    conn.action({ do: 'todo.toggle', id });
  }

  function remove(id: number) {
    buzz();
    conn.action({ do: 'todo.remove', id });
  }
</script>

<div class="card todo">
  <div class="head">
    <span class="label">{title}</span>
    <span class="count">{open}</span>
  </div>

  <ul>
    {#each items as item (item.id)}
      <li class:done={item.done}>
        <button class="tick" aria-label={item.done ? 'Vrátit' : 'Hotovo'} onclick={() => toggle(item.id)}>
          {#if item.done}<Check size="70%" strokeWidth={3.2} />{/if}
        </button>
        <button class="text" onclick={() => toggle(item.id)}>{item.text}</button>
        {#if item.done}
          <button class="drop" aria-label="Smazat" onclick={() => remove(item.id)}><X size="3.4vmin" strokeWidth={2.4} /></button>
        {/if}
      </li>
    {:else}
      <li class="empty">Nic na seznamu</li>
    {/each}
  </ul>

  <form onsubmit={add}>
    <input type="text" placeholder="Nový úkol" maxlength="200" enterkeyhint="done" bind:value={draft} />
    <button type="submit" aria-label="Přidat" disabled={!draft.trim()}><Plus size="4.4vmin" strokeWidth={2.6} /></button>
  </form>
</div>

<style>
  .todo {
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .count {
    font-size: 2.6vmin;
    font-weight: 700;
    color: var(--dim);
  }
  ul {
    list-style: none;
    margin: 1.2vmin 0 0;
    padding: 0;
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: none;
  }
  ul::-webkit-scrollbar {
    display: none;
  }
  li {
    display: flex;
    align-items: center;
    min-height: 7.4vmin;
    border-bottom: 1px solid var(--line);
  }
  li:last-child {
    border-bottom: 0;
  }
  .tick {
    width: 4.6vmin;
    height: 4.6vmin;
    flex: none;
    margin-right: 2vmin;
    border-radius: 50%;
    border: 0.4vmin solid var(--mute);
    display: grid;
    place-items: center;
    color: #fff;
    transition: background 0.2s, border-color 0.2s, transform 0.3s var(--spring);
  }
  .tick:active {
    transform: scale(0.8);
  }
  .done .tick {
    background: var(--accent);
    border-color: var(--accent);
  }
  .text {
    flex: 1;
    min-width: 0;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 1.6vmin 0;
  }
  .done .text {
    color: var(--mute);
    text-decoration: line-through;
  }
  .drop {
    width: 6vmin;
    height: 6vmin;
    flex: none;
    display: grid;
    place-items: center;
    color: var(--mute);
  }
  .empty {
    color: var(--mute);
    border: 0;
  }
  form {
    display: flex;
    align-items: center;
    margin-top: 1.2vmin;
    flex: none;
  }
  input {
    flex: 1;
    min-width: 0;
    height: 7.6vmin;
    padding: 0 2.4vmin;
    border: 0;
    border-radius: 2vmin;
    background: var(--line);
    color: var(--text);
    font: inherit;
    outline: none;
    user-select: text;
    -webkit-user-select: text;
  }
  input::placeholder {
    color: var(--mute);
  }
  form button {
    width: 7.6vmin;
    height: 7.6vmin;
    flex: none;
    margin-left: 1.4vmin;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--accent);
    color: #fff;
    transition: opacity 0.2s, transform 0.3s var(--spring);
  }
  form button:disabled {
    opacity: 0.35;
  }
  form button:active {
    transform: scale(0.88);
  }
</style>
