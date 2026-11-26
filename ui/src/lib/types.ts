// Shapes shared with the agent (crates/agent/src/protocol.rs, config.rs).

import type { Background, ThemeDef, ThemeInput, WidgetStyle } from './theme';

export type Action =
  | { do: 'launch'; target: string; args?: string }
  | { do: 'keys'; combo: string }
  | { do: 'media.play_pause' }
  | { do: 'media.next' }
  | { do: 'media.prev' }
  | { do: 'audio.mute_toggle' }
  | { do: 'audio.set_volume'; level: number }
  | { do: 'system.lock' }
  | { do: 'display.off' }
  | { do: 'timer.toggle'; focus?: number; rest?: number }
  | { do: 'timer.reset' }
  | { do: 'timer.skip' }
  | { do: 'todo.add'; text: string }
  | { do: 'todo.toggle'; id: number }
  | { do: 'todo.remove'; id: number }
  | { do: 'todo.clear_done' }
  | { do: 'process.kill'; name: string }
  | { do: 'ai.decide'; request: number; allow: boolean };

/**
 * How a deck key is drawn: `icon` = coloured tile with a white glyph,
 * `glyph` = bare coloured icon, `tinted` = icon on a wash of its colour,
 * `filled` = the whole key in the colour.
 */
export type KeyLook = 'icon' | 'glyph' | 'tinted' | 'filled';

export interface DeckButton {
  label: string;
  icon?: string;
  /** Big text instead of an icon ("BRB", "1"…). */
  text?: string;
  /** Small line above the text or icon ("SCENE"). */
  caption?: string;
  /** Coloured status dot above the text or icon. */
  dot?: string;
  look?: KeyLook;
  /** Colour of the icon. */
  color?: string;
  /** Colour of the button itself. */
  background?: string;
  labelColor?: string;
  /** Picture from the assets folder shown instead of the icon. */
  image?: string;
  action: Action;
}

export interface WidgetDef {
  id: string;
  type: string;
  x: number;
  y: number;
  w: number;
  h: number;
  config?: Record<string, any>;
  style?: WidgetStyle;
}

export interface PageDef {
  id: string;
  name: string;
  widgets: WidgetDef[];
  theme?: ThemeInput;
  background?: Background | string | null;
}

export interface Config {
  port: number;
  openOnConnect: boolean;
  pages: PageDef[];
  theme?: ThemeInput;
  /** The user's own named themes. */
  themes?: Record<string, ThemeDef>;
  /** Page indicator at the bottom edge. */
  dots?: boolean;
  night?: NightMode;
  /** Phones may connect over Wi-Fi after pairing. */
  wireless?: boolean;
  /** A language code, or "auto" (the default) for each device's own language. */
  language?: string;
}

/** Dims the dashboard at night, e.g. for a phone on the bedside table. */
export interface NightMode {
  enabled?: boolean;
  /** "HH:MM", default 22:00. */
  from?: string;
  /** "HH:MM", default 07:00. */
  to?: string;
  /** 0–1, how much darker. */
  dim?: number;
  /** Red-only light, easier on sleepy eyes. */
  red?: boolean;
  /** Id of the page to turn to when the night starts. */
  page?: string;
}

export interface AiSession {
  id: string;
  /** Which tool it belongs to: `claude`, `codex`… */
  source: string;
  project: string;
  title: string;
  status: 'idle' | 'working' | 'waiting' | 'done';
  detail: string;
  updatedAt: number;
  /** A permission question the phone may answer. */
  request?: { id: number; tool: string; detail: string };
}

export interface DiscordMember {
  name: string;
  status: 'online' | 'idle' | 'dnd' | string;
  activity?: string;
  /** Voice channel they sit in. */
  voice?: string;
  muted: boolean;
  avatar?: string;
}

export interface DiscordServer {
  id: string;
  name: string;
  online: number;
  members: DiscordMember[];
  problem?: 'disabled' | 'unknown' | 'offline';
}

export interface ModelServer {
  name: string;
  /** `lmstudio`, `ollama` or `openai` (anything OpenAI-compatible). */
  kind: string;
  online: boolean;
  models: { name: string; info: string }[];
  installed: number | null;
}

export interface AiSessions {
  sessions: AiSession[];
  finished: number;
  asked: number;
}

export interface TimerState {
  phase: 'focus' | 'break';
  running: boolean;
  remainingMs: number;
  totalMs: number;
  rounds: number;
  finished: number;
}

export interface TodoItem {
  id: number;
  text: string;
  done: boolean;
}

export interface ProcessRow {
  name: string;
  cpu: number;
  mem: number;
  count: number;
  killable: boolean;
}

export interface SystemStats {
  cpu: number;
  memUsed: number;
  memTotal: number;
  netRx: number;
  netTx: number;
}

export interface MediaNow {
  title: string;
  artist: string;
  album: string;
  app: string;
  playing: boolean;
  positionMs: number;
  durationMs: number;
  art: string | null;
}

export interface Volume {
  level: number;
  muted: boolean;
}
