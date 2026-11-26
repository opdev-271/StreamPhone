<script lang="ts">
  import { untrack } from 'svelte';
  import { ArrowDown, ArrowUp } from '@lucide/svelte';
  import { conn } from '../conn.svelte';
  import type { SystemStats, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  $effect(() => conn.subscribe('system.stats'));

  const stats = $derived(conn.data['system.stats'] as SystemStats | undefined);

  // CPU history for the graph.
  let history = $state<number[]>([]);
  $effect(() => {
    const cpu = stats?.cpu;
    if (cpu === undefined) return;
    // Reading `history` here must not re-trigger this effect.
    untrack(() => (history = [...history.slice(-59), cpu]));
  });

  const line = $derived(
    history.map((v, i) => `${(i / 59) * 100},${30 - (Math.min(v, 100) / 100) * 27}`).join(' '),
  );
  const area = $derived(history.length > 1 ? `0,30 ${line} ${((history.length - 1) / 59) * 100},30` : '');

  const gb = (bytes: number) => (bytes / 1073741824).toFixed(1).replace('.', ',');

  function rate(bytesPerSec: number) {
    const kb = bytesPerSec / 1024;
    if (kb < 1000) return `${Math.round(kb)} kB/s`;
    return `${(kb / 1024).toFixed(1).replace('.', ',')} MB/s`;
  }

  const level = (pct: number) => (pct > 90 ? 'var(--bad)' : pct > 70 ? 'var(--warn)' : 'var(--ok)');
  const memPct = $derived(stats ? (stats.memUsed / stats.memTotal) * 100 : 0);

  // Ring geometry: radius 42 in a 100×100 box.
  const LEN = 2 * Math.PI * 42;
  const dash = (pct: number) => `${(Math.min(100, Math.max(0, pct)) / 100) * LEN} ${LEN}`;
  const graph = $derived(def.config?.graph !== false);
</script>

<div class="card sys">
  <div class="label">Počítač</div>
  {#if stats}
    <div class="rings">
      {#each [{ name: 'CPU', pct: stats.cpu }, { name: 'RAM', pct: memPct }] as r (r.name)}
        <div class="ring">
          <div class="dial">
            <svg viewBox="0 0 100 100" aria-hidden="true">
              <circle class="track" cx="50" cy="50" r="42" />
              <circle class="value" cx="50" cy="50" r="42" style="stroke:{level(r.pct)};stroke-dasharray:{dash(r.pct)}" />
            </svg>
            <div class="pct"><span>{Math.round(r.pct)}<small>%</small></span></div>
          </div>
          <div class="name">{r.name}</div>
        </div>
      {/each}
    </div>

    {#if graph}
      <svg class="spark" viewBox="0 0 100 30" preserveAspectRatio="none" aria-hidden="true">
        <defs>
          <linearGradient id="sys-fade" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0" stop-color="var(--accent)" stop-opacity="0.45" />
            <stop offset="1" stop-color="var(--accent)" stop-opacity="0" />
          </linearGradient>
        </defs>
        <polygon points={area} fill="url(#sys-fade)" />
        <polyline points={line} fill="none" stroke="var(--accent)" stroke-width="1.6" stroke-linejoin="round" vector-effect="non-scaling-stroke" />
      </svg>
    {/if}

    <div class="foot">
      <span class="mem">{gb(stats.memUsed)} / {gb(stats.memTotal)} GB</span>
      <span class="net"><ArrowDown size="2.8vmin" strokeWidth={2.6} />{rate(stats.netRx)}</span>
      <span class="net"><ArrowUp size="2.8vmin" strokeWidth={2.6} />{rate(stats.netTx)}</span>
    </div>
  {:else}
    <div class="dim">Načítám…</div>
  {/if}
</div>

<style>
  .sys {
    display: flex;
    flex-direction: column;
  }
  .rings {
    display: flex;
    justify-content: space-around;
    margin-top: 2vmin;
    flex: none;
  }
  .ring {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 0;
  }
  .dial {
    position: relative;
    width: 17vmin;
    height: 17vmin;
  }
  .dial svg {
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }
  circle {
    fill: none;
    stroke-width: 11;
    stroke-linecap: round;
  }
  .track {
    stroke: var(--line);
  }
  .value {
    transition: stroke-dasharray 0.7s ease, stroke 0.6s;
  }
  .pct {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    display: grid;
    place-items: center;
    font-size: 4.8vmin;
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .pct small {
    font-size: 0.5em;
    font-weight: 600;
    color: var(--dim);
    margin-left: 0.1em;
  }
  .name {
    margin-top: 1vmin;
    font-size: 2.5vmin;
    font-weight: 700;
    color: var(--dim);
    letter-spacing: 0.05em;
  }
  .spark {
    flex: 1 1 0;
    min-height: 0;
    width: 100%;
    margin-top: 1.4vmin;
  }
  .foot {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    margin-top: auto;
    padding-top: 1.2vmin;
    font-size: 2.6vmin;
    font-weight: 600;
    color: var(--dim);
    white-space: nowrap;
  }
  .mem {
    flex: 0 0 100%;
    margin-bottom: 0.6vmin;
  }
  .net {
    display: inline-flex;
    align-items: center;
  }
  .net :global(svg) {
    margin-right: 0.5vmin;
    color: var(--mute);
  }
</style>
