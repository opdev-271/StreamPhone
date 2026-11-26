<script lang="ts">
  import { buzz, conn } from '../conn.svelte';
  import { icons } from '../icons';
  import { assetUrl, color, withOpacity } from '../theme';
  import type { DeckButton, KeyLook, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  const buttons = $derived((def.config?.buttons ?? []) as DeckButton[]);
  const cols = $derived((def.config?.cols as number) ?? 6);
  const rows = $derived(Math.max(1, Math.ceil(buttons.length / cols)));
  const labels = $derived(def.config?.labels !== false);
  // Unless set, the picture grows with the key: roughly 38 % of its height.
  const iconSize = $derived.by(() => {
    const set = def.config?.iconSize as number | undefined;
    if (set) return set;
    const keyHeight = ((def.h / 6) * 93 - (rows - 1) * 2) / rows;
    return Math.round(Math.min(15, Math.max(6.5, keyHeight * 0.38)) * 10) / 10;
  });
  const deckLook = $derived((def.config?.look as KeyLook) ?? 'icon');

  const lookOf = (b: DeckButton): KeyLook => b.look ?? deckLook;

  function style(b: DeckButton) {
    const look = lookOf(b);
    const tint = color(b.color);
    const vars: string[] = [];
    // A coloured tile needs some colour; a bare glyph follows the text colour.
    if (tint) vars.push(`--tint:${tint}`);
    else if (look !== 'glyph') vars.push('--tint:#636366');
    if (look === 'tinted') vars.push(`--wash:${withOpacity(tint ?? '#8e8e93', 0.2)}`);
    const bg = color(b.background);
    if (bg) vars.push(`--key:${bg}`);
    const label = color(b.labelColor);
    if (label) vars.push(`--label:${label}`);
    vars.push(`--icon:${iconSize}vmin`);
    return vars.join(';');
  }

  let failed = $state<number | null>(null);

  async function press(i: number) {
    buzz();
    const res = await conn.press(def.id, i);
    if (!res.ok) {
      failed = i;
      buzz(60);
      setTimeout(() => failed === i && (failed = null), 900);
    }
  }
</script>

<div class="deck" style="grid-template-columns:repeat({cols},minmax(0,1fr));grid-template-rows:repeat({rows},minmax(0,1fr))">
  {#each buttons as b, i (i)}
    {@const Icon = icons[b.icon ?? 'square'] ?? icons.square}
    <button
      class="key {lookOf(b)}"
      class:failed={failed === i}
      style={style(b)}
      aria-label={b.label || b.text}
      onclick={() => press(i)}
    >
      {#if b.dot}
        <span class="dot" style="color:{color(b.dot)}"></span>
      {/if}
      {#if b.caption}
        <span class="caption">{b.caption}</span>
      {/if}
      {#if b.text}
        <span class="text" class:long={b.text.length > 3}>{b.text}</span>
      {:else if b.image}
        <img class="tile pic" src={assetUrl(b.image)} alt="" draggable="false" />
      {:else}
        <span class="tile"><Icon size="58%" strokeWidth={2.1} /></span>
      {/if}
      {#if labels && b.label}
        <span class="name">{b.label}</span>
      {/if}
    </button>
  {/each}
</div>

<style>
  .deck {
    display: grid;
    grid-gap: var(--gap);
    min-width: 0;
    min-height: 0;
  }
  .key {
    --tint: var(--text);
    position: relative;
    background: var(--sheen), var(--key, var(--card));
    border: 1px solid var(--line);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    -webkit-backdrop-filter: var(--card-blur);
    backdrop-filter: var(--card-blur);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    padding: 1vmin;
    transition: transform 0.4s var(--spring), filter 0.2s;
  }
  .key:active {
    transform: scale(0.93);
    filter: brightness(1.3);
    transition: transform 0.07s, filter 0.07s;
  }
  .key.failed {
    border-color: var(--bad);
    animation: shake 0.3s;
  }

  /* The picture on a key: by default a coloured rounded tile with a white
     glyph, like an app icon. */
  .tile {
    width: var(--icon, 9.4vmin);
    height: var(--icon, 9.4vmin);
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 27%;
    color: #fff;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.3) 0%, rgba(255, 255, 255, 0) 70%), var(--tint);
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.28), 0 0.8vmin 1.8vmin rgba(0, 0, 0, 0.35);
  }
  .pic {
    object-fit: cover;
    background: none;
    box-shadow: 0 0.8vmin 1.8vmin rgba(0, 0, 0, 0.35);
  }

  /* glyph: just the icon in its colour. */
  .glyph .tile:not(.pic) {
    background: none;
    box-shadow: none;
    color: var(--tint);
  }
  .glyph .tile:not(.pic) :global(svg) {
    width: 86%;
    height: 86%;
  }

  /* tinted: the icon in its colour on a wash of the same colour. */
  .tinted {
    background: var(--sheen), var(--wash);
  }
  .tinted .tile:not(.pic) {
    background: none;
    box-shadow: none;
    color: var(--tint);
  }
  .tinted .tile:not(.pic) :global(svg) {
    width: 86%;
    height: 86%;
  }

  /* filled: the whole key is the colour. */
  .filled {
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.26) 0%, rgba(255, 255, 255, 0) 70%), var(--tint);
    border-color: rgba(255, 255, 255, 0.18);
    --label: rgba(255, 255, 255, 0.92);
  }
  .filled .tile:not(.pic) {
    background: none;
    box-shadow: none;
  }
  .filled .tile:not(.pic) :global(svg) {
    width: 86%;
    height: 86%;
  }
  .filled .text,
  .filled .caption {
    color: #fff;
  }

  /* Text keys: SCENE 1, BRB, LIVE… */
  .text {
    font-size: 8.6vmin;
    font-weight: 800;
    line-height: 1;
    letter-spacing: -0.02em;
    color: var(--tint);
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
  }
  .text.long {
    font-size: 5.6vmin;
    letter-spacing: 0;
  }
  .caption {
    font-size: 2.1vmin;
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: var(--accent);
    margin-bottom: 0.8vmin;
  }
  .dot {
    width: 1.5vmin;
    height: 1.5vmin;
    border-radius: 50%;
    margin-bottom: 1.2vmin;
    background: currentColor;
    box-shadow: 0 0 1.4vmin currentColor;
  }
  .name {
    margin-top: 1.5vmin;
    font-size: 2.6vmin;
    font-weight: 600;
    color: var(--label, var(--dim));
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
