<script lang="ts">
  import type { PageDef, WidgetDef } from '../lib/types';

  export type Rect = { x: number; y: number; w: number; h: number };

  // Sits exactly over the dashboard in the preview and mirrors its 12 × 6
  // grid, so a box here covers the widget it stands for.
  let {
    page,
    gap,
    width,
    height,
    selected,
    names,
    onselect,
    onchange,
  }: {
    page: PageDef;
    /** Space between widgets, in the dashboard's units (1 = 1 % of the height). */
    gap: number;
    width: number;
    height: number;
    selected: string | null;
    names: Record<string, string>;
    onselect: (id: string | null) => void;
    onchange: (id: string, rect: Rect) => void;
  } = $props();

  const COLS = 12;
  const ROWS = 6;

  // Same numbers as `.page` in Page.svelte.
  const unit = $derived(height / 100);
  const side = $derived(2.5 * unit);
  const bottom = $derived(4.5 * unit);
  const space = $derived(gap * unit);
  const stepX = $derived((width - 2 * side - (COLS - 1) * space) / COLS + space);
  const stepY = $derived((height - side - bottom - (ROWS - 1) * space) / ROWS + space);

  const clamp = (n: number, low: number, high: number) => Math.min(high, Math.max(low, n));

  let drag: { id: string; resize: boolean; startX: number; startY: number; from: Rect } | null = null;

  function down(e: PointerEvent, w: WidgetDef, resize: boolean) {
    e.preventDefault();
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { id: w.id, resize, startX: e.clientX, startY: e.clientY, from: { x: w.x, y: w.y, w: w.w, h: w.h } };
    onselect(w.id);
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    const dx = Math.round((e.clientX - drag.startX) / stepX);
    const dy = Math.round((e.clientY - drag.startY) / stepY);
    const f = drag.from;
    const next: Rect = drag.resize
      ? { x: f.x, y: f.y, w: clamp(f.w + dx, 1, COLS - f.x), h: clamp(f.h + dy, 1, ROWS - f.y) }
      : { x: clamp(f.x + dx, 0, COLS - f.w), y: clamp(f.y + dy, 0, ROWS - f.h), w: f.w, h: f.h };
    onchange(drag.id, next);
  }

  function up() {
    drag = null;
  }

  const overlaps = (a: Rect, b: Rect) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
  const clashing = $derived(
    new Set(page.widgets.filter((a) => page.widgets.some((b) => b !== a && overlaps(a, b))).map((w) => w.id)),
  );
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="grid"
  style="padding:{side}px {side}px {bottom}px;grid-gap:{space}px"
  onpointerdown={() => onselect(null)}
>
  {#each Array(COLS * ROWS) as _, i (i)}
    <i class="cell" style="grid-column:{(i % COLS) + 1};grid-row:{Math.floor(i / COLS) + 1}"></i>
  {/each}
  {#each page.widgets as w (w.id)}
    <div
      class="box"
      class:on={selected === w.id}
      class:clash={clashing.has(w.id)}
      style="grid-column:{w.x + 1} / span {w.w};grid-row:{w.y + 1} / span {w.h}"
      onpointerdown={(e) => down(e, w, false)}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
    >
      <span>{names[w.type] ?? w.type}</span>
      <small>{w.w} × {w.h}</small>
      <b class="grip" title="Změnit velikost" onpointerdown={(e) => down(e, w, true)} onpointermove={move} onpointerup={up} onpointercancel={up}
      ></b>
    </div>
  {/each}
</div>

<style>
  .grid {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    display: grid;
    grid-template-columns: repeat(12, minmax(0, 1fr));
    grid-template-rows: repeat(6, minmax(0, 1fr));
    background: rgba(0, 0, 0, 0.45);
    user-select: none;
    touch-action: none;
  }
  .cell {
    border: 1px dashed rgba(255, 255, 255, 0.14);
    border-radius: 6px;
  }
  .box {
    position: relative;
    z-index: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    border: 2px solid rgba(255, 255, 255, 0.55);
    border-radius: 12px;
    background: rgba(10, 132, 255, 0.16);
    color: #fff;
    font-weight: 700;
    cursor: grab;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .box:active {
    cursor: grabbing;
  }
  .box.on {
    border-color: #0a84ff;
    background: rgba(10, 132, 255, 0.34);
    z-index: 2;
  }
  .box.clash {
    border-color: #ff453a;
    background: rgba(255, 69, 58, 0.28);
  }
  .box small {
    font-weight: 600;
    opacity: 0.7;
  }
  .grip {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 22px;
    height: 22px;
    cursor: nwse-resize;
    border-radius: 10px 0 10px 0;
    background: rgba(255, 255, 255, 0.75);
  }
  .box.on .grip {
    background: #0a84ff;
  }
</style>
