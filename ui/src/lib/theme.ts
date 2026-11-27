// Look of the dashboard: built-in themes, the user's own themes from
// config.json, wallpapers, and per-page / per-widget overrides. Everything ends
// up as CSS variables, so a page or a widget restyles just its own subtree.

export interface Colors {
  bg: string;
  card: string;
  cardHi: string;
  line: string;
  text: string;
  dim: string;
  mute: string;
  accent: string;
  ok: string;
  warn: string;
  bad: string;
}

export interface Background {
  /** File in the assets folder, or a full URL. */
  image?: string;
  fit?: 'cover' | 'contain' | 'stretch' | 'tile';
  /** 0–1: how much the image is darkened so text stays readable. */
  dim?: number;
  /** Blur strength, 0 = sharp. */
  blur?: number;
  /** CSS gradient or colour drawn instead of / under the image. */
  fill?: string;
}

export interface CardLook {
  /** 0–1: see-through cards let the wallpaper show. */
  opacity?: number;
  /** Frosted glass behind see-through cards. */
  blur?: number;
  /** Corner radius, 0 = square. */
  radius?: number;
  border?: boolean;
  /** Soft highlight and shadow that lift the cards off the background. */
  depth?: boolean;
}

export interface FontLook {
  family?: string;
  /** Font file in the assets folder (.ttf, .otf, .woff, .woff2). */
  file?: string;
}

export interface ThemeDef {
  /** Built-in theme or one of the user's own (`themes` in config.json). */
  preset?: string;
  colors?: Partial<Colors>;
  background?: Background | string | null;
  card?: CardLook;
  font?: FontLook;
  /** Space between widgets. */
  gap?: number;
}

/** A theme is given by name or spelled out. */
export type ThemeInput = ThemeDef | string | null | undefined;

/** Per-widget look (`style` on a widget in config.json). */
export interface WidgetStyle {
  accent?: string;
  text?: string;
  background?: string;
  opacity?: number;
  radius?: number;
  border?: boolean;
}

export interface Theme {
  colors: Colors;
  background: Background | null;
  card: Required<CardLook>;
  font: FontLook;
  gap: number;
}

const midnight: Theme = {
  colors: {
    bg: '#000000',
    card: '#1c1c1e',
    cardHi: '#2c2c2e',
    line: '#ffffff17',
    text: '#f5f5f7',
    dim: '#a1a1a8',
    mute: '#6e6e76',
    accent: '#0a84ff',
    ok: '#30d158',
    warn: '#ff9f0a',
    bad: '#ff453a',
  },
  background: null,
  card: { opacity: 1, blur: 0, radius: 4.2, border: true, depth: true },
  font: {},
  gap: 2,
};

/** Built-in themes, as differences from `midnight`. */
export const presets: Record<string, ThemeDef> = {
  midnight: {},
  graphite: {
    colors: { bg: '#101113', card: '#1a1c1f', cardHi: '#25282c', line: '#30343a', text: '#eceef0', dim: '#a3a8b0', mute: '#6c727b', accent: '#7cb7ff' },
  },
  ocean: {
    colors: { bg: '#04111c', card: '#0a1f30', cardHi: '#123049', line: '#1b3f5c', text: '#e6f3fb', dim: '#93b4c8', mute: '#5d7f95', accent: '#38c6f4', ok: '#4fe0c0' },
  },
  forest: {
    colors: { bg: '#060d08', card: '#0f1b12', cardHi: '#18291c', line: '#243a29', text: '#ecf3e6', dim: '#a3b59b', mute: '#6b7d65', accent: '#8fd15a', ok: '#8fd15a' },
  },
  sunset: {
    colors: { bg: '#140a12', card: '#22111d', cardHi: '#33192a', line: '#47233a', text: '#fbeee6', dim: '#c9a79f', mute: '#8c6c6c', accent: '#ff8a5c', warn: '#ffc857' },
    background: { fill: 'linear-gradient(135deg, #2a0f2e 0%, #5b1d3a 55%, #b5482f 100%)' },
    card: { opacity: 0.6, blur: 2 },
  },
  rose: {
    colors: { bg: '#12080d', card: '#1f1017', cardHi: '#2e1823', line: '#412233', text: '#fbe9f0', dim: '#c79fb0', mute: '#8a6676', accent: '#ff6fa3' },
  },
  paper: {
    colors: { bg: '#f1eee6', card: '#fbfaf6', cardHi: '#e9e5da', line: '#d9d4c7', text: '#1d1c19', dim: '#5f5c54', mute: '#8f8b80', accent: '#5a48d6', ok: '#1f9d74', warn: '#c57a0c', bad: '#d1413b' },
  },
  terminal: {
    colors: { bg: '#000000', card: '#030a04', cardHi: '#0a1c0d', line: '#14491c', text: '#4dff6a', dim: '#2fc04a', mute: '#1c7a2e', accent: '#4dff6a', ok: '#4dff6a', warn: '#e6ff4d', bad: '#ff5a4d' },
    card: { radius: 0.4, depth: false },
    font: { family: "'Cascadia Mono', 'Roboto Mono', Consolas, monospace" },
  },
  neon: {
    colors: { bg: '#07030f', card: '#120a22', cardHi: '#1d1136', line: '#3a1f6b', text: '#f3eaff', dim: '#b9a3e0', mute: '#7a66a8', accent: '#ff3df2', ok: '#2ff3e0', warn: '#ffe14d', bad: '#ff4d6d' },
  },
  // The security looks: a console in an operations room. Square cards, a
  // monospaced face and a faint grid behind see-through panels.
  cyber: {
    colors: { bg: '#02070d', card: '#06131f', cardHi: '#0c2233', line: '#00e5ff38', text: '#d8f6ff', dim: '#7fb6c9', mute: '#48728a', accent: '#00e5ff', ok: '#00ff9c', warn: '#ffd000', bad: '#ff2a6d' },
    background: {
      fill: 'linear-gradient(#00e5ff12 1px, transparent 1px) 0 0 / 7vmin 7vmin, linear-gradient(90deg, #00e5ff12 1px, transparent 1px) 0 0 / 7vmin 7vmin, radial-gradient(ellipse at 50% 0%, #06304a 0%, #02070d 70%)',
    },
    card: { opacity: 0.78, blur: 1.2, radius: 0.6, depth: false },
    font: { family: "'Cascadia Mono', 'JetBrains Mono', 'Roboto Mono', Consolas, monospace" },
  },
  redteam: {
    colors: { bg: '#080000', card: '#140506', cardHi: '#260a0c', line: '#ff2a2a40', text: '#ffe1de', dim: '#d08e8a', mute: '#8a4d4b', accent: '#ff2a2a', ok: '#ffb000', warn: '#ffb000', bad: '#ff2a2a' },
    background: {
      fill: 'repeating-linear-gradient(0deg, #ff2a2a0d 0 1px, transparent 1px 0.9vmin), radial-gradient(ellipse at 50% 100%, #3a0508 0%, #080000 70%)',
    },
    card: { opacity: 0.8, blur: 1, radius: 0.6, depth: false },
    font: { family: "'Cascadia Mono', 'JetBrains Mono', 'Roboto Mono', Consolas, monospace" },
  },
  blueteam: {
    colors: { bg: '#030816', card: '#081328', cardHi: '#102042', line: '#3d7bff40', text: '#e2ecff', dim: '#93a9d6', mute: '#566a99', accent: '#3d7bff', ok: '#2fe6a7', warn: '#ffc53d', bad: '#ff4d6d' },
    background: {
      fill: 'radial-gradient(#3d7bff26 1px, transparent 1.4px) 0 0 / 5vmin 5vmin, linear-gradient(160deg, #06173a 0%, #030816 60%)',
    },
    card: { opacity: 0.8, blur: 1.2, radius: 1, depth: false },
    font: { family: "'Cascadia Mono', 'JetBrains Mono', 'Roboto Mono', Consolas, monospace" },
  },
  glass: {
    background: { fill: 'linear-gradient(140deg, #1b2a4a 0%, #3b2a5c 50%, #16404a 100%)' },
    card: { opacity: 0.45, blur: 2.5, radius: 3.4 },
    colors: { card: '#20222b', cardHi: '#3a3d4a', line: '#ffffff26' },
  },
};

export const presetNames = Object.keys(presets);

/** Colours that can be written by name wherever a colour is expected. */
export const palette: Record<string, string> = {
  red: '#ff453a',
  orange: '#ff9f0a',
  yellow: '#ffd60a',
  green: '#30d158',
  mint: '#63e6e2',
  teal: '#40c8e0',
  cyan: '#64d2ff',
  blue: '#0a84ff',
  indigo: '#5e5ce6',
  purple: '#bf5af2',
  pink: '#ff375f',
  brown: '#ac8e68',
  gray: '#8e8e93',
  graphite: '#48484a',
  white: '#ffffff',
  black: '#000000',
};

/** A palette name or any CSS colour. */
export function color(value: string | undefined): string | undefined {
  return value ? (palette[value.toLowerCase()] ?? value) : undefined;
}

function normalizeBackground(bg: Background | string | null | undefined): Background | null | undefined {
  if (bg === undefined) return undefined;
  if (bg === null || bg === '') return null;
  return typeof bg === 'string' ? { image: bg } : bg;
}

/**
 * Applies `input` on top of `base`. Named themes are looked up among the
 * user's own first, then the built-in ones; a theme may itself build on a
 * preset.
 */
export function applyTheme(base: Theme, input: ThemeInput, own: Record<string, ThemeDef> = {}, depth = 0): Theme {
  if (!input || depth > 8) return base;
  if (typeof input === 'string') {
    const named = own[input] ?? presets[input];
    return named ? applyTheme(base, named, own, depth + 1) : base;
  }
  let out = base;
  if (input.preset) {
    // A user theme may reuse the name of the built-in one it builds on.
    const mine = own[input.preset];
    const named = mine && mine !== input ? mine : presets[input.preset];
    if (named) out = applyTheme(out, named, own, depth + 1);
  }
  const bg = normalizeBackground(input.background);
  return {
    colors: { ...out.colors, ...input.colors },
    background: bg === undefined ? out.background : bg,
    card: { ...out.card, ...input.card },
    font: { ...out.font, ...input.font },
    gap: input.gap ?? out.gap,
  };
}

export function resolveTheme(input: ThemeInput, own?: Record<string, ThemeDef>): Theme {
  return applyTheme(midnight, input, own ?? {});
}

/** `#rgb`, `#rrggbb` or `#rrggbbaa` with the given opacity; other notations pass through. */
export function withOpacity(color: string, opacity: number): string {
  if (opacity >= 1) return color;
  const hex = /^#([0-9a-f]{3,8})$/i.exec(color.trim())?.[1];
  if (!hex || (hex.length !== 3 && hex.length !== 6 && hex.length !== 8)) return color;
  const full = hex.length === 3 ? hex.replace(/./g, '$&$&') : hex;
  const n = (i: number) => parseInt(full.slice(i, i + 2), 16);
  const own = full.length === 8 ? n(6) / 255 : 1;
  return `rgba(${n(0)},${n(2)},${n(4)},${(Math.max(0, opacity) * own).toFixed(3)})`;
}

/** Where a file from the assets folder (or a full URL) is fetched from. */
export function assetUrl(file: string): string {
  if (/^(https?:|data:|\/)/i.test(file)) return file;
  return '/user/' + file.split(/[\\/]/).map(encodeURIComponent).join('/');
}

export const CUSTOM_FONT = 'StreamPhoneCustom';

function fontFamily(font: FontLook): string | null {
  const parts = [font.file ? `'${CUSTOM_FONT}'` : null, font.family || null].filter(Boolean);
  return parts.length ? `${parts.join(', ')}, system-ui, sans-serif` : null;
}

/** The theme as CSS variables (inline style of the element it applies to). */
export function themeVars(theme: Theme): string {
  const { colors: c, card } = theme;
  const vars: Record<string, string> = {
    '--bg': c.bg,
    '--card': withOpacity(c.card, card.opacity),
    '--card-hi': withOpacity(c.cardHi, Math.min(1, card.opacity + 0.15)),
    '--line': card.border ? c.line : 'transparent',
    '--text': c.text,
    '--dim': c.dim,
    '--mute': c.mute,
    '--accent': c.accent,
    '--ok': c.ok,
    '--warn': c.warn,
    '--bad': c.bad,
    '--radius': `${card.radius}vmin`,
    '--gap': `${theme.gap}vmin`,
    '--card-blur': card.blur > 0 && card.opacity < 1 ? `blur(${card.blur}vmin)` : 'none',
  };
  if (!card.depth) {
    vars['--sheen'] = 'none';
    vars['--shadow'] = 'none';
  }
  const family = fontFamily(theme.font);
  if (family) vars['--font'] = family;
  return toStyle(vars);
}

/** One widget's overrides on top of the theme around it. */
export function widgetVars(theme: Theme, style: WidgetStyle | undefined): string {
  if (!style) return '';
  const vars: Record<string, string> = {};
  const accent = color(style.accent);
  const text = color(style.text);
  if (accent) vars['--accent'] = accent;
  if (text) vars['--text'] = text;
  if (style.background !== undefined || style.opacity !== undefined) {
    const opacity = style.opacity ?? theme.card.opacity;
    vars['--card'] = withOpacity(color(style.background) ?? theme.colors.card, opacity);
    if (opacity >= 1) vars['--card-blur'] = 'none';
  }
  if (style.radius !== undefined) vars['--radius'] = `${style.radius}vmin`;
  if (style.border !== undefined) vars['--line'] = style.border ? theme.colors.line : 'transparent';
  return toStyle(vars);
}

function toStyle(vars: Record<string, string>): string {
  return Object.keys(vars)
    .map((k) => `${k}:${vars[k]}`)
    .join(';');
}

let loadedFont = '';

/** Loads the theme's font file, once per file. */
export function loadFont(font: FontLook) {
  const file = font.file ?? '';
  if (file === loadedFont) return;
  loadedFont = file;
  if (!file || typeof FontFace === 'undefined') return;
  new FontFace(CUSTOM_FONT, `url("${assetUrl(file)}")`)
    .load()
    .then((face) => (document as any).fonts.add(face))
    .catch(() => {
      /* missing or broken file: the fallback font stays */
    });
}
