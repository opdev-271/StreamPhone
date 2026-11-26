<script lang="ts">
  import Backdrop from './Backdrop.svelte';
  import { applyTheme, resolveTheme, themeVars, widgetVars, type Theme, type ThemeDef } from './theme';
  import type { PageDef } from './types';
  import { widgets } from './widgets';

  let {
    page,
    active,
    theme,
    themes,
  }: { page: PageDef; active: boolean; theme: Theme; themes: Record<string, ThemeDef> } = $props();

  // A page may switch to another theme and/or bring its own wallpaper. Its
  // theme replaces the global one outright (no half-inherited glass or
  // wallpaper); a wallpaper alone keeps the global theme.
  const own = $derived(page.theme != null || page.background !== undefined);
  const look = $derived.by(() => {
    if (!own) return theme;
    const themed = page.theme != null ? resolveTheme(page.theme, themes) : theme;
    return page.background !== undefined ? applyTheme(themed, { background: page.background }, themes) : themed;
  });
</script>

<section class="page" style={own ? themeVars(look) : ''}>
  {#if own}
    <Backdrop background={look.background} />
  {/if}
  {#if active}
    {#each page.widgets as w (w.id)}
      {@const Widget = widgets[w.type]}
      <div
        class="cell"
        style="grid-column:{w.x + 1} / span {w.w};grid-row:{w.y + 1} / span {w.h};{widgetVars(look, w.style)}"
      >
        {#if Widget}
          <Widget def={w} />
        {:else}
          <div class="card unknown">Neznámý widget „{w.type}“</div>
        {/if}
      </div>
    {/each}
  {/if}
</section>

<style>
  .page {
    position: relative;
    flex: 0 0 100%;
    height: 100%;
    scroll-snap-align: start;
    scroll-snap-stop: always;
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    grid-template-rows: repeat(6, minmax(0, 1fr));
    grid-gap: var(--gap);
    padding: 2.5vmin 2.5vmin 4.5vmin;
    padding-left: max(2.5vmin, env(safe-area-inset-left));
    padding-right: max(2.5vmin, env(safe-area-inset-right));
    color: var(--text);
    font-family: var(--font);
  }
  .cell {
    position: relative;
    z-index: 1;
    min-width: 0;
    min-height: 0;
    display: grid;
    color: var(--text);
  }
  .unknown {
    display: grid;
    place-items: center;
    color: var(--mute);
  }
</style>
