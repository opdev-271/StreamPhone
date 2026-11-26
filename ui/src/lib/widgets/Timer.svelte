<script lang="ts">
  import { t } from '../strings';
  import { Pause, Play, RotateCcw, SkipForward } from '@lucide/svelte';
  import { buzz, conn } from '../conn.svelte';
  import type { TimerState, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  $effect(() => conn.subscribe('timer.state'));

  const timer = $derived(conn.data['timer.state'] as TimerState | undefined);
  const focus = $derived((def.config?.focus as number) ?? 25);
  const rest = $derived((def.config?.rest as number) ?? 5);

  const left = $derived(timer ? timer.remainingMs : focus * 60000);
  const total = $derived(timer ? timer.totalMs : focus * 60000);
  const resting = $derived(timer?.phase === 'break');

  const time = $derived.by(() => {
    const s = Math.ceil(left / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  });

  // Ring geometry: radius 44 in a 100×100 box. It empties as time runs out.
  const LEN = 2 * Math.PI * 44;
  const dash = $derived(`${(total > 0 ? left / total : 1) * LEN} ${LEN}`);

  // A phase ran out: let the hand on the desk know.
  let seen: number | undefined;
  $effect(() => {
    const n = timer?.finished;
    if (n === undefined) return;
    if (seen !== undefined && n > seen) {
      try {
        navigator.vibrate?.([220, 120, 220, 120, 420]);
      } catch {
        /* not supported */
      }
    }
    seen = n;
  });

  function tap(action: 'timer.toggle' | 'timer.reset' | 'timer.skip') {
    buzz();
    conn.action(action === 'timer.toggle' ? { do: action, focus, rest } : { do: action });
  }
</script>

<div class="card timer" class:resting>
  <div class="label">{resting ? t('Přestávka') : t('Soustředění')}</div>

  <div class="dial">
    <svg viewBox="0 0 100 100" aria-hidden="true">
      <circle class="track" cx="50" cy="50" r="44" />
      <circle class="value" cx="50" cy="50" r="44" style="stroke-dasharray:{dash}" />
    </svg>
    <div class="time">{time}</div>
  </div>

  <div class="rounds" aria-label={t('Dokončená kola: {n}', { n: timer?.rounds ?? 0 })}>
    {#each Array(4) as _, i (i)}
      <i class:on={((timer?.rounds ?? 0) % 4 || ((timer?.rounds ?? 0) > 0 ? 4 : 0)) > i}></i>
    {/each}
  </div>

  <div class="controls">
    <button aria-label={t('Od začátku')} onclick={() => tap('timer.reset')}><RotateCcw size="4.6vmin" strokeWidth={2.2} /></button>
    <button class="main" aria-label={timer?.running ? t('Pozastavit') : t('Spustit')} onclick={() => tap('timer.toggle')}>
      {#if timer?.running}
        <Pause size="5.4vmin" fill="currentColor" strokeWidth={0} />
      {:else}
        <Play size="5.4vmin" fill="currentColor" strokeWidth={1.5} />
      {/if}
    </button>
    <button aria-label={t('Přeskočit')} onclick={() => tap('timer.skip')}><SkipForward size="4.6vmin" strokeWidth={2.2} /></button>
  </div>
</div>

<style>
  .timer {
    display: flex;
    flex-direction: column;
    align-items: center;
    --tone: var(--accent);
  }
  .resting {
    --tone: var(--ok);
  }
  .label {
    align-self: flex-start;
  }
  .dial {
    position: relative;
    flex: 1 1 0;
    min-height: 0;
    width: 100%;
    margin: 1.4vmin 0;
  }
  .dial svg {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    transform: rotate(-90deg);
  }
  circle {
    fill: none;
    stroke-width: 6;
    stroke-linecap: round;
  }
  .track {
    stroke: var(--line);
  }
  .value {
    stroke: var(--tone);
    transition: stroke-dasharray 1s linear, stroke 0.4s;
  }
  .time {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    display: grid;
    place-items: center;
    font-size: 8.4vmin;
    font-weight: 600;
    letter-spacing: -0.04em;
  }
  .rounds {
    display: flex;
    margin-bottom: 1.6vmin;
  }
  .rounds i {
    width: 1.4vmin;
    height: 1.4vmin;
    margin: 0 0.6vmin;
    border-radius: 50%;
    background: var(--line);
    transition: background 0.3s;
  }
  .rounds i.on {
    background: var(--tone);
  }
  .controls {
    display: flex;
    align-items: center;
    color: var(--dim);
  }
  .controls button {
    width: 9vmin;
    height: 9vmin;
    margin: 0 1.2vmin;
    border-radius: 50%;
    display: grid;
    place-items: center;
    transition: transform 0.4s var(--spring), filter 0.2s;
  }
  .controls button:active {
    transform: scale(0.86);
    filter: brightness(1.4);
    transition: transform 0.07s;
  }
  .controls .main {
    width: 11vmin;
    height: 11vmin;
    color: #fff;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.28), rgba(255, 255, 255, 0) 70%), var(--tone);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.28), 0 0.8vmin 2vmin rgba(0, 0, 0, 0.35);
  }
</style>
