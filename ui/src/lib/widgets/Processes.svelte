<script lang="ts">
  import { number } from '../i18n.svelte';
  import { t } from '../strings';
  import { buzz, conn } from '../conn.svelte';
  import type { ProcessRow, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  $effect(() => conn.subscribe('system.processes'));

  const byMemory = $derived(def.config?.sort === 'mem');
  const canKill = $derived(def.config?.kill !== false);

  const rows = $derived.by(() => {
    const list = [...((conn.data['system.processes'] as ProcessRow[] | undefined) ?? [])];
    list.sort((a, b) => (byMemory ? b.mem - a.mem : b.cpu - a.cpu || b.mem - a.mem));
    return list.slice(0, 12);
  });

  const title = (name: string) => name.replace(/\.exe$/i, '');

  function size(bytes: number) {
    const mb = bytes / 1048576;
    return mb < 1000 ? `${Math.round(mb)} MB` : `${number(mb / 1024, 1)} GB`;
  }

  // Ending a program takes two taps: the first one only arms the row.
  let armed = $state<string | null>(null);
  let failed = $state<string | null>(null);
  let disarm: ReturnType<typeof setTimeout> | undefined;

  function arm(row: ProcessRow) {
    if (!canKill || !row.killable) return;
    buzz();
    armed = armed === row.name ? null : row.name;
    clearTimeout(disarm);
    disarm = setTimeout(() => (armed = null), 4000);
  }

  async function end(e: Event, name: string) {
    e.stopPropagation();
    buzz(30);
    armed = null;
    const res = await conn.action({ do: 'process.kill', name });
    if (!res.ok) {
      failed = name;
      setTimeout(() => failed === name && (failed = null), 1200);
    }
  }
</script>

<div class="card procs">
  <div class="head">
    <span class="label">{t('Procesy')}</span>
    <span class="label">{byMemory ? t('Paměť') : 'CPU'}</span>
  </div>
  <div class="list">
    {#each rows as row (row.name)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="row" class:armed={armed === row.name} class:failed={failed === row.name} onclick={() => arm(row)}>
        <span class="name">
          {title(row.name)}{#if row.count > 1}<small>{row.count}</small>{/if}
        </span>
        {#if armed === row.name}
          <button class="end" onclick={(e) => end(e, row.name)}>{t('Ukončit')}</button>
        {:else}
          <span class="mem">{size(row.mem)}</span>
          <span class="cpu">{number(row.cpu, 1)} %</span>
        {/if}
        <i style="width:{Math.min(100, byMemory ? 0 : row.cpu)}%"></i>
      </div>
    {:else}
      <div class="dim">{t('Načítám…')}</div>
    {/each}
  </div>
</div>

<style>
  .procs {
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    justify-content: space-between;
  }
  .list {
    flex: 1 1 0;
    min-height: 0;
    overflow: hidden;
    margin-top: 1vmin;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    height: 6.6vmin;
    border-bottom: 1px solid var(--line);
    font-size: 3.1vmin;
    white-space: nowrap;
  }
  .row i {
    position: absolute;
    left: 0;
    bottom: -1px;
    height: 0.5vmin;
    border-radius: 1vmin;
    background: var(--accent);
    transition: width 1s ease;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 600;
  }
  .name small {
    margin-left: 1vmin;
    padding: 0 1.1vmin;
    border-radius: 2vmin;
    background: var(--line);
    color: var(--dim);
    font-size: 0.75em;
    font-weight: 700;
  }
  .mem,
  .cpu {
    flex: none;
    text-align: right;
    color: var(--dim);
  }
  .mem {
    width: 13vmin;
  }
  .cpu {
    width: 11vmin;
    color: var(--text);
    font-weight: 600;
  }
  .armed {
    color: var(--bad);
  }
  .failed {
    animation: shake 0.3s;
  }
  .end {
    flex: none;
    height: 5vmin;
    padding: 0 2.4vmin;
    border-radius: 3vmin;
    background: var(--bad);
    color: #fff;
    font-weight: 700;
    font-size: 2.8vmin;
  }
  @keyframes shake {
    25% {
      transform: translateX(-0.8vmin);
    }
    75% {
      transform: translateX(0.8vmin);
    }
  }
</style>
