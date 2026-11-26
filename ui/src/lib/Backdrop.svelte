<script lang="ts">
  import { assetUrl, type Background } from './theme';

  // `fixed` covers the whole screen behind every page; otherwise it fills the
  // page it sits in.
  let { background, fixed = false }: { background: Background | null; fixed?: boolean } = $props();

  const fit = $derived(background?.fit ?? 'cover');
  const size = $derived(fit === 'stretch' ? '100% 100%' : fit === 'tile' ? 'auto' : fit);
  const blur = $derived(background?.blur ?? 0);
  const dim = $derived(Math.min(1, Math.max(0, background?.dim ?? 0)));
</script>

<div class="backdrop" class:fixed aria-hidden="true">
  {#if background?.fill}
    <div class="layer" style="background:{background.fill}"></div>
  {/if}
  {#if background?.image}
    <div
      class="layer"
      class:blurred={blur > 0}
      style="background-image:url('{assetUrl(background.image)}');background-size:{size};background-repeat:{fit === 'tile'
        ? 'repeat'
        : 'no-repeat'};{blur > 0 ? `filter:blur(${blur}vmin)` : ''}"
    ></div>
  {/if}
  {#if dim > 0}
    <div class="layer" style="background:rgba(0,0,0,{dim})"></div>
  {/if}
</div>

<style>
  .backdrop {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    overflow: hidden;
    background: var(--bg);
    pointer-events: none;
    z-index: 0;
  }
  .backdrop.fixed {
    position: fixed;
  }
  .layer {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    background-position: center;
  }
  /* Blur fades to transparent at the edges; overscan hides that. */
  .blurred {
    top: -4vmin;
    right: -4vmin;
    bottom: -4vmin;
    left: -4vmin;
  }
</style>
