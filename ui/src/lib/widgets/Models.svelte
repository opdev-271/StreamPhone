<script lang="ts">
  import { conn } from '../conn.svelte';
  import type { ModelServer, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  $effect(() => conn.subscribe('ai.models'));

  const servers = $derived(conn.data['ai.models'] as ModelServer[] | undefined);
  const title = $derived((def.config?.title as string) || 'Lokální modely');
  // Servers that are switched off only take up room; hide them on request.
  const shown = $derived((servers ?? []).filter((s) => s.online || def.config?.hideOffline !== true));

  function summary(s: ModelServer) {
    if (!s.online) return 'Neběží';
    // OpenAI-compatible servers only say what they offer, not what is loaded.
    if (s.kind !== 'lmstudio' && s.kind !== 'ollama') return `${s.models.length} k dispozici`;
    return s.models.length ? `${s.models.length} v paměti` : 'Nic v paměti';
  }
</script>

<div class="card models">
  <div class="label">{title}</div>
  <div class="list">
    {#if !servers}
      <div class="quiet">Zjišťuji…</div>
    {:else}
      {#each shown as s (s.name)}
        <div class="server" class:off={!s.online} class:busy={s.online && s.models.length > 0}>
          <div class="line">
            <i class="state"></i>
            <span class="name">{s.name}</span>
            <span class="sum">{summary(s)}</span>
          </div>
          {#each s.models as m (m.name)}
            <div class="model">
              <span class="id">{m.name}</span>
              {#if m.info}<span class="info">{m.info}</span>{/if}
            </div>
          {/each}
          {#if s.online && s.installed != null && s.installed > 0}
            <div class="disk">{s.installed} na disku</div>
          {/if}
        </div>
      {:else}
        <div class="quiet">Žádný server neběží</div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .models {
    display: flex;
    flex-direction: column;
  }
  .list {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    margin-top: 1vmin;
    scrollbar-width: none;
  }
  .list::-webkit-scrollbar {
    display: none;
  }
  .server {
    --tone: var(--ok);
    padding: 1.6vmin 0;
    border-bottom: 1px solid var(--line);
  }
  .server:last-child {
    border-bottom: 0;
  }
  .off {
    --tone: var(--mute);
  }
  .busy {
    --tone: var(--accent);
  }
  .line {
    display: flex;
    align-items: center;
    white-space: nowrap;
  }
  .state {
    width: 1.8vmin;
    height: 1.8vmin;
    flex: none;
    margin-right: 1.6vmin;
    border-radius: 50%;
    background: var(--tone);
  }
  .name {
    font-weight: 700;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .off .name {
    color: var(--dim);
  }
  .sum {
    margin-left: auto;
    padding-left: 1.6vmin;
    font-size: 2.6vmin;
    font-weight: 700;
    color: var(--tone);
    flex: none;
  }
  .model {
    display: flex;
    align-items: baseline;
    margin: 0.6vmin 0 0 3.4vmin;
    white-space: nowrap;
    font-size: 3vmin;
  }
  .id {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: ui-monospace, 'Cascadia Mono', 'Roboto Mono', Consolas, monospace;
    font-size: 2.7vmin;
  }
  .info {
    margin-left: auto;
    padding-left: 1.6vmin;
    color: var(--dim);
    font-size: 2.5vmin;
    flex: none;
  }
  .disk {
    margin: 0.6vmin 0 0 3.4vmin;
    color: var(--mute);
    font-size: 2.5vmin;
  }
  .quiet {
    color: var(--mute);
    padding-top: 1.4vmin;
  }
</style>
