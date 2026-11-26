<script lang="ts">
  import { conn } from './lib/conn.svelte';
  import Page from './lib/Page.svelte';
  import Offline from './lib/Offline.svelte';
  import Backdrop from './lib/Backdrop.svelte';
  import { keepAwake } from './lib/awake';
  import { setLanguage } from './lib/i18n.svelte';
  import { loadFont, resolveTheme, themeVars } from './lib/theme';

  let pager = $state<HTMLDivElement>();
  let current = $state(0);

  conn.connect();

  // The language chosen on the PC, or the phone's own when it says "auto".
  $effect(() => setLanguage(conn.config?.language));

  const themes = $derived(conn.config?.themes ?? {});
  const theme = $derived(resolveTheme(conn.config?.theme, themes));

  // The theme lives on <html>, so it also covers the offline screen.
  $effect(() => {
    const root = document.documentElement;
    const pairs = themeVars(theme)
      .split(';')
      .map((pair) => [pair.slice(0, pair.indexOf(':')), pair.slice(pair.indexOf(':') + 1)]);
    for (const [name, value] of pairs) root.style.setProperty(name, value);
    loadFont(theme.font);
    return () => pairs.forEach(([name]) => root.style.removeProperty(name));
  });

  // A page removed from the config must not leave us scrolled past the end.
  $effect(() => {
    const count = conn.config?.pages.length ?? 0;
    if (count && current >= count) current = count - 1;
  });

  // Sideways (portrait) the pager's axis runs down the screen, and browsers do
  // not scroll a rotated element by touch reliably, so page it ourselves.
  let touch: { x: number; y: number } | null = null;
  const sideways = () => window.matchMedia('(orientation: portrait)').matches;

  function onTouchStart(e: TouchEvent) {
    const el = e.target as HTMLElement;
    touch = sideways() && el.tagName !== 'INPUT' ? { x: e.touches[0].clientX, y: e.touches[0].clientY } : null;
  }

  function onTouchEnd(e: TouchEvent) {
    if (!touch) return;
    const dx = e.changedTouches[0].clientX - touch.x;
    const dy = e.changedTouches[0].clientY - touch.y;
    touch = null;
    if (Math.abs(dy) > 60 && Math.abs(dy) > Math.abs(dx) * 1.5) go(current + (dy < 0 ? 1 : -1));
  }

  function onScroll() {
    if (!pager) return;
    current = Math.round(pager.scrollLeft / pager.clientWidth);
  }

  function go(i: number) {
    const count = conn.config?.pages.length ?? 0;
    if (!pager || i < 0 || i >= count) return;
    pager.scrollTo({ left: i * pager.clientWidth, behavior: 'smooth' });
  }

  // Night mode: darker (optionally red-only) between two times of day.
  let minute = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (minute = Date.now()), 20000);
    return () => clearInterval(t);
  });
  const night = $derived.by(() => {
    const n = conn.config?.night;
    if (!n?.enabled) return null;
    const at = (text: string | undefined, fallback: number) => {
      const [h, m] = (text ?? '').split(':').map(Number);
      return Number.isFinite(h) ? h * 60 + (m || 0) : fallback;
    };
    const d = new Date(minute);
    const now = d.getHours() * 60 + d.getMinutes();
    const from = at(n.from, 22 * 60);
    const to = at(n.to, 7 * 60);
    // The span usually crosses midnight.
    const dark = from <= to ? now >= from && now < to : now >= from || now < to;
    return dark ? n : null;
  });

  // Turn to the bedside page once, when the night starts.
  let wasNight = false;
  $effect(() => {
    if (!pager) return;
    const target = night?.page;
    if (night && !wasNight && target) {
      const index = conn.config?.pages.findIndex((p) => p.id === target) ?? -1;
      if (index >= 0) go(index);
    }
    wasNight = !!night;
  });

  // The editor shows the dashboard in a frame and asks it to turn to the page
  // being edited. Turning pages is all a message can do.
  function onMessage(e: MessageEvent) {
    if (e.source !== window.parent || e.source === window) return;
    if (e.data?.type === 'streamphone:page') go(Number(e.data.index));
  }

  function onKey(e: KeyboardEvent) {
    if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') return;
    // Otherwise the browser also scrolls the pager itself and a page is skipped.
    e.preventDefault();
    go(current + (e.key === 'ArrowRight' ? 1 : -1));
  }

  // Keep the phone's screen on while the dashboard is visible.
  $effect(() => keepAwake());
</script>

<svelte:window onkeydown={onKey} onmessage={onMessage} />

{#if conn.config}
  <Backdrop background={theme.background} fixed />
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="pager" bind:this={pager} onscroll={onScroll} ontouchstart={onTouchStart} ontouchend={onTouchEnd}>
    {#each conn.config.pages as page, i (page.id)}
      <Page {page} {theme} {themes} active={Math.abs(i - current) <= 1} />
    {/each}
  </div>
  {#if conn.config.pages.length > 1 && conn.config.dots !== false}
    <div class="dots">
      {#each conn.config.pages as page, i (page.id)}
        <button class="dot" class:on={i === current} aria-label={page.name} onclick={() => go(i)}></button>
      {/each}
    </div>
  {/if}
{/if}

{#if night}
  {#if night.red}
    <div class="night red"></div>
  {/if}
  <div class="night" style="background:rgba(0,0,0,{Math.min(0.95, Math.max(0, night.dim ?? 0.5))})"></div>
{/if}

{#if conn.status !== 'online'}
  <Offline />
{/if}

<style>
  .pager {
    position: relative;
    z-index: 1;
    height: 100%;
    display: flex;
    overflow-x: auto;
    overflow-y: hidden;
    scroll-snap-type: x mandatory;
    scrollbar-width: none;
  }
  .pager::-webkit-scrollbar {
    display: none;
  }
  .dots {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    height: 4vmin;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    z-index: 2;
  }
  .night {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    pointer-events: none;
    z-index: 5;
  }
  /* Multiplying by red removes the green and blue light. */
  .night.red {
    background: #ff3a1a;
    mix-blend-mode: multiply;
  }
  .dot {
    pointer-events: auto;
    width: 1.2vmin;
    height: 1.2vmin;
    margin: 0 0.8vmin;
    opacity: 0.35;
    border-radius: 1vmin;
    background: var(--text);
    transition: opacity 0.25s, transform 0.25s;
  }
  .dot.on {
    opacity: 1;
    transform: scale(1.15);
  }
</style>
