<script lang="ts">
  import { date as formatDate } from '../i18n.svelte';
  import type { WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  // Re-read the date now and then so the highlighted day moves at midnight.
  let now = $state(new Date());
  $effect(() => {
    const t = setInterval(() => (now = new Date()), 60000);
    return () => clearInterval(t);
  });

  const mondayFirst = $derived(def.config?.weekStart !== 'sunday');
  // Day names in the dashboard's language; 2024-01-01 was a Monday.
  const names = $derived(
    Array.from({ length: 7 }, (_, i) => formatDate(new Date(2024, 0, (mondayFirst ? 1 : 7) + i), { weekday: 'short' }).replace('.', '').slice(0, 2)),
  );

  const title = $derived.by(() => {
    const text = formatDate(now, { month: 'long', year: 'numeric' });
    return text.charAt(0).toUpperCase() + text.slice(1);
  });

  // Six weeks of day numbers; 0 marks a cell outside the month.
  const cells = $derived.by(() => {
    const year = now.getFullYear();
    const month = now.getMonth();
    const first = new Date(year, month, 1).getDay();
    const lead = mondayFirst ? (first + 6) % 7 : first;
    const days = new Date(year, month + 1, 0).getDate();
    const out: number[] = [];
    for (let i = 0; i < lead; i++) out.push(0);
    for (let d = 1; d <= days; d++) out.push(d);
    while (out.length % 7) out.push(0);
    return out;
  });

  const weekend = (index: number) => (mondayFirst ? index % 7 >= 5 : index % 7 === 0 || index % 7 === 6);
</script>

<div class="card cal">
  <div class="label">{title}</div>
  <div class="grid" style="grid-template-rows:auto repeat({cells.length / 7},minmax(0,1fr))">
    {#each names as name, i (i)}
      <span class="head" class:weekend={weekend(i)}>{name}</span>
    {/each}
    {#each cells as day, i (i)}
      <span class="day" class:weekend={weekend(i)} class:today={day === now.getDate()}>
        {#if day}<b>{day}</b>{/if}
      </span>
    {/each}
  </div>
</div>

<style>
  .cal {
    display: flex;
    flex-direction: column;
  }
  .grid {
    flex: 1 1 0;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(7, minmax(0, 1fr));
    margin-top: 1.4vmin;
    text-align: center;
  }
  .head {
    font-size: 2.3vmin;
    font-weight: 700;
    color: var(--mute);
    padding-bottom: 0.8vmin;
  }
  .day {
    display: grid;
    place-items: center;
    font-size: 3.1vmin;
    font-weight: 600;
    min-height: 0;
  }
  .day b {
    font-weight: inherit;
    display: grid;
    place-items: center;
    width: 5.6vmin;
    height: 5.6vmin;
    max-height: 100%;
    border-radius: 50%;
  }
  .weekend {
    color: var(--dim);
  }
  .today b {
    background: var(--accent);
    color: #fff;
    font-weight: 700;
  }
</style>
