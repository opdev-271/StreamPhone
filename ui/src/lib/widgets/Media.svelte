<script lang="ts">
  import { Music, Pause, Play, SkipBack, SkipForward, Volume1, Volume2, VolumeX } from '@lucide/svelte';
  import { buzz, conn } from '../conn.svelte';
  import type { MediaNow, Volume, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  $effect(() => conn.subscribe('media.now'));
  $effect(() => conn.subscribe('audio.volume'));

  const media = $derived(conn.data['media.now'] as MediaNow | null | undefined);
  const volume = $derived(conn.data['audio.volume'] as Volume | null | undefined);

  // The cover, blurred, colours the whole card (off with "artBackground": false).
  const glow = $derived(def.config?.artBackground !== false && !!media?.art);

  const progress = $derived(
    media && media.durationMs > 0 ? Math.min(100, (media.positionMs / media.durationMs) * 100) : 0,
  );

  function clock(ms: number) {
    const s = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  }

  function tap(name: 'media.prev' | 'media.play_pause' | 'media.next' | 'audio.mute_toggle') {
    buzz();
    conn.action({ do: name });
  }

  // While the finger is on the slider, show the local value and send at most ~12×/s.
  let dragging = $state<number | null>(null);
  let lastSent = 0;
  function onVolume(e: Event) {
    const level = Number((e.currentTarget as HTMLInputElement).value) / 100;
    dragging = level;
    const now = Date.now();
    if (now - lastSent > 80) {
      lastSent = now;
      conn.action({ do: 'audio.set_volume', level });
    }
  }
  function onVolumeEnd() {
    if (dragging !== null) conn.action({ do: 'audio.set_volume', level: dragging });
    setTimeout(() => (dragging = null), 600);
  }

  const shown = $derived(Math.round((dragging ?? volume?.level ?? 0) * 100));
</script>

<div class="card media" class:glow>
  {#if glow && media?.art}
    <img class="wash" src="/art/{media.art}" alt="" aria-hidden="true" />
    <div class="shade"></div>
  {/if}

  <div class="top">
    <div class="art">
      {#if media?.art}
        <img src="/art/{media.art}" alt="" />
      {:else}
        <Music size="42%" strokeWidth={1.8} />
      {/if}
    </div>
    <div class="meta">
      {#if media}
        <div class="title">{media.title || 'Neznámá skladba'}</div>
        <div class="artist">{media.artist}</div>
      {:else}
        <div class="title idle">Nic nehraje</div>
      {/if}
    </div>
  </div>

  <div class="seek">
    <div class="bar"><i style="width:{progress}%"></i></div>
    <div class="times">
      <span>{media ? clock(media.positionMs) : ''}</span>
      <span>{media && media.durationMs > 0 ? clock(media.durationMs) : ''}</span>
    </div>
  </div>

  <div class="controls">
    <button aria-label="Předchozí" onclick={() => tap('media.prev')}>
      <SkipBack size="6.4vmin" fill="currentColor" strokeWidth={1.5} />
    </button>
    <button class="main" aria-label="Přehrát / pauza" onclick={() => tap('media.play_pause')}>
      {#if media?.playing}
        <Pause size="9vmin" fill="currentColor" strokeWidth={0} />
      {:else}
        <Play size="9vmin" fill="currentColor" strokeWidth={1.5} />
      {/if}
    </button>
    <button aria-label="Další" onclick={() => tap('media.next')}>
      <SkipForward size="6.4vmin" fill="currentColor" strokeWidth={1.5} />
    </button>
  </div>

  {#if volume}
    <div class="vol">
      <button aria-label="Ztlumit" class:muted={volume.muted} onclick={() => tap('audio.mute_toggle')}>
        {#if volume.muted}
          <VolumeX size="4.4vmin" fill="currentColor" />
        {:else}
          <Volume1 size="4.4vmin" fill="currentColor" />
        {/if}
      </button>
      <input
        type="range"
        min="0"
        max="100"
        value={shown}
        aria-label="Hlasitost"
        oninput={onVolume}
        onchange={onVolumeEnd}
        style="--fill:{shown}%"
      />
      <span class="loud"><Volume2 size="4.4vmin" fill="currentColor" /></span>
    </div>
  {/if}
</div>

<style>
  .media {
    position: relative;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    --track: var(--line);
  }
  /* Over the cover the card is always dark, whatever the theme. */
  .glow {
    --text: #fff;
    --dim: rgba(255, 255, 255, 0.72);
    --mute: rgba(255, 255, 255, 0.55);
    --track: rgba(255, 255, 255, 0.24);
    color: #fff;
  }
  .wash {
    position: absolute;
    top: -20%;
    left: -20%;
    width: 140%;
    height: 140%;
    object-fit: cover;
    filter: blur(7vmin) saturate(1.5);
  }
  .shade {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    background: linear-gradient(180deg, rgba(0, 0, 0, 0.35), rgba(0, 0, 0, 0.6));
  }
  .top,
  .seek,
  .controls,
  .vol {
    position: relative;
  }
  .top {
    display: flex;
    align-items: center;
    min-height: 0;
  }
  .art {
    width: 14vmin;
    height: 14vmin;
    flex: none;
    border-radius: 2.2vmin;
    background: var(--card-hi);
    color: var(--mute);
    display: grid;
    place-items: center;
    overflow: hidden;
    margin-right: 2.8vmin;
    box-shadow: 0 1vmin 2.6vmin rgba(0, 0, 0, 0.45);
  }
  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .meta {
    min-width: 0;
  }
  .title,
  .artist {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .title {
    font-size: 4vmin;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .title.idle {
    color: var(--dim);
    font-weight: 500;
  }
  .artist {
    color: var(--dim);
    font-size: 3.3vmin;
    margin-top: 0.3vmin;
  }
  .bar {
    height: 0.9vmin;
    border-radius: 1vmin;
    background: var(--track);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--text);
    border-radius: 1vmin;
    transition: width 1s linear;
  }
  .times {
    display: flex;
    justify-content: space-between;
    font-size: 2.3vmin;
    font-weight: 600;
    color: var(--mute);
    min-height: 3vmin;
    margin-top: 0.8vmin;
  }
  .controls {
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .controls button {
    width: 11vmin;
    height: 11vmin;
    margin: 0 2.4vmin;
    border-radius: 50%;
    display: grid;
    place-items: center;
    transition: transform 0.4s var(--spring), background 0.25s;
  }
  .controls button:active {
    transform: scale(0.86);
    background: var(--track);
    transition: transform 0.07s, background 0.07s;
  }
  .vol {
    display: flex;
    align-items: center;
    color: var(--dim);
  }
  .vol button,
  .loud {
    display: grid;
    place-items: center;
    width: 6vmin;
    height: 7vmin;
    flex: none;
  }
  .vol .muted {
    color: var(--bad);
  }
  input[type='range'] {
    flex: 1;
    min-width: 0;
    height: 7vmin;
    margin: 0 1vmin;
    background: none;
    -webkit-appearance: none;
    appearance: none;
  }
  input[type='range']::-webkit-slider-runnable-track {
    height: 1.2vmin;
    border-radius: 1vmin;
    background: linear-gradient(to right, var(--text) var(--fill), var(--track) var(--fill));
  }
  input[type='range']::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 3.8vmin;
    height: 3.8vmin;
    margin-top: -1.3vmin;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0.4vmin 1.2vmin rgba(0, 0, 0, 0.45);
  }
</style>
