<script lang="ts">
  import { date as formatDate } from '../i18n.svelte';
  import type { WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  let now = $state(new Date());
  $effect(() => {
    const t = setInterval(() => (now = new Date()), 1000);
    return () => clearInterval(t);
  });

  const big = $derived(def.h >= 5);
  const two = (n: number) => String(n).padStart(2, '0');
  // 24-hour unless the widget asks for 12h; the phone's locale is not followed,
  // because an unexpected "PM" does not fit the card.
  const half = $derived(def.config?.format === '12h');
  const hours = $derived(half ? String(now.getHours() % 12 || 12) : two(now.getHours()));
  const time = $derived(`${hours}:${two(now.getMinutes())}`);
  const showDate = $derived(def.config?.date !== false);

  // Optional look: size, weight, colour, alignment and font of the digits.
  const look = $derived.by(() => {
    const c = def.config ?? {};
    const css: string[] = [];
    // Unless set, the digits grow and shrink with the widget.
    css.push(`--clock-size:${c.size ?? Math.min(40, def.w * 4.2, def.h * 7.5)}vmin`);
    if (c.weight) css.push(`--clock-weight:${c.weight}`);
    if (c.color) css.push(`--clock-color:${c.color}`);
    if (c.font) css.push(`--clock-font:${c.font}`);
    if (c.align) css.push(`--clock-align:${c.align === 'center' ? 'center' : c.align === 'right' ? 'flex-end' : 'flex-start'}`);
    return css.join(';');
  });
  const secs = $derived(two(now.getSeconds()));
  const date = $derived(formatDate(now, { weekday: 'long', day: 'numeric', month: 'long' }));
</script>

<div class="card clock" class:big style={look}>
  <div class="time">
    {time}{#if def.config?.seconds !== false}<span class="secs">{secs}</span>{/if}{#if half}<span class="secs"
        >{now.getHours() < 12 ? 'AM' : 'PM'}</span
      >{/if}
  </div>
  {#if showDate}
    <div class="date dim">{date}</div>
  {/if}
</div>

<style>
  .clock {
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: var(--clock-align, flex-start);
  }
  .time {
    font-size: var(--clock-size, 17vmin);
    font-weight: var(--clock-weight, 500);
    color: var(--clock-color, inherit);
    font-family: var(--clock-font, inherit);
    line-height: 1;
    letter-spacing: -0.045em;
    white-space: nowrap;
  }
  .secs {
    font-size: 0.3em;
    color: var(--mute);
    margin-left: 0.3em;
    letter-spacing: 0;
    font-weight: 600;
  }
  .date {
    margin-top: calc(var(--clock-size, 17vmin) * 0.12);
    font-size: calc(var(--clock-size, 17vmin) * 0.2);
    white-space: nowrap;
    font-weight: 600;
  }
  .date::first-letter {
    text-transform: uppercase;
  }
  .big {
    align-items: var(--clock-align, center);
    background: none;
    border-color: transparent;
  }
  .big .time {
    font-size: var(--clock-size, 40vmin);
  }
  .big .date {
    font-size: 5vmin;
    margin-top: 4vmin;
  }
</style>
