import { t } from './strings';
import { hmacSha256Hex } from './hmac';
import type { Action, Config } from './types';

type Result = { ok: boolean; error?: string };

/** What a paired phone remembers about this PC. */
type Pairing = { device: string; secret: string };

const STORE = 'streamphone.pairing';

function readPairing(): Pairing | null {
  try {
    const saved = JSON.parse(localStorage.getItem(STORE) ?? 'null');
    return saved?.device && saved?.secret ? saved : null;
  } catch {
    return null;
  }
}

function writePairing(pairing: Pairing | null) {
  try {
    if (pairing) localStorage.setItem(STORE, JSON.stringify(pairing));
    else localStorage.removeItem(STORE);
  } catch {
    /* private mode: the phone will have to pair again next time */
  }
}

/** The one-time code from the QR link (`…/#pair=CODE`). */
function pairCode(): string | null {
  return /[#&]pair=([A-Za-z0-9]+)/.exec(location.hash)?.[1] ?? null;
}

/**
 * An Android browser that is not the StreamPhone app (the app marks itself in
 * the user agent). That is where a QR code read by the phone's camera lands.
 */
const outsideApp = /Android/.test(navigator.userAgent) && !/StreamPhoneApp\//.test(navigator.userAgent);

/** A name for the list of paired phones on the PC, e.g. "Pixel 4a". */
function deviceName(): string {
  const model = /Android [\d.]+; ([^;)]+?)(?: Build|\))/.exec(navigator.userAgent)?.[1];
  if (model && model !== 'K') return model.trim();
  if (/iPhone|iPad/.test(navigator.userAgent)) return /iPad/.test(navigator.userAgent) ? 'iPad' : 'iPhone';
  return /Android/.test(navigator.userAgent) ? t('Telefon s Androidem') : t('Prohlížeč');
}

/**
 * One WebSocket to the agent. Widgets subscribe to topics (ref-counted), the
 * agent only computes topics somebody is subscribed to.
 *
 * Over USB (and on the PC itself) the agent welcomes the page right away.
 * Over Wi-Fi it first asks for proof of pairing.
 */
class Connection {
  /** `unpaired`: reached the PC over Wi-Fi, but this phone is not paired. */
  status = $state<'connecting' | 'online' | 'offline' | 'unpaired'>('connecting');
  config = $state<Config | null>(null);
  data = $state<Record<string, any>>({});
  /**
   * A link that hands the pairing code over to the StreamPhone app. Set while
   * the page holds an unused code in a phone's browser: the code works once,
   * so the browser must not spend it before the person has chosen.
   */
  appLink = $state<string | null>(null);
  private stayHere = false;
  /** A pairing code typed on the phone instead of scanned. */
  private typed: string | null = null;
  /** The typed code was wrong, used or too old. */
  codeRefused = $state(false);

  /**
   * Pair with the code shown on the PC. For where the QR link cannot reach:
   * an iPhone's home-screen app keeps its own storage, apart from Safari's.
   */
  pair(code: string) {
    this.typed = code.replace(/[^A-Za-z0-9]/g, '').toUpperCase();
    this.codeRefused = false;
    this.status = 'connecting';
    clearTimeout(this.retry);
    if (!this.ws) this.open();
  }

  /** Pair this browser after all, instead of the app. */
  useBrowser() {
    this.stayHere = true;
    this.appLink = null;
    this.status = 'connecting';
    // Do not wait for the slow retry of an unpaired phone.
    clearTimeout(this.retry);
    if (!this.ws) this.open();
  }

  private retry: ReturnType<typeof setTimeout> | undefined;

  private ws: WebSocket | null = null;
  // Always listening for config changes made on the PC.
  private refs = new Map<string, number>([['agent.config', 1]]);
  private pending = new Map<number, (r: Result) => void>();
  private nextId = 1;
  private retryMs = 500;
  private started = false;
  private welcomed = false;

  connect() {
    if (this.started) return;
    this.started = true;
    this.open();
  }

  private open() {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    const ws = new WebSocket(`${proto}://${location.host}/ws`);
    this.ws = ws;
    this.welcomed = false;
    let refused = false;

    ws.onopen = () => {
      this.retryMs = 500;
    };

    ws.onmessage = (ev) => {
      let msg: any;
      try {
        msg = JSON.parse(ev.data);
      } catch {
        return;
      }
      switch (msg.t) {
        case 'hello': {
          // Wi-Fi: prove the pairing, or pair now with the code from the QR.
          const pairing = readPairing();
          const code = this.typed ?? pairCode();
          // A typed code wins over an old pairing the PC may have forgotten.
          if (this.typed) this.send({ t: 'pair', code: this.typed, name: deviceName() });
          else if (pairing) this.send({ t: 'auth', device: pairing.device, proof: hmacSha256Hex(pairing.secret, String(msg.nonce)) });
          else if (code && outsideApp && !this.stayHere) {
            refused = true;
            this.status = 'unpaired';
            this.appLink = `streamphone://pair?h=${encodeURIComponent(location.host)}&c=${code}`;
            ws.close();
          } else if (code) this.send({ t: 'pair', code, name: deviceName() });
          else {
            refused = true;
            this.status = 'unpaired';
            ws.close();
          }
          break;
        }
        case 'paired':
          writePairing({ device: msg.device, secret: msg.secret });
          this.appLink = null;
          this.typed = null;
          // The code is spent; keep it out of the address and the history.
          history.replaceState(null, '', location.pathname + location.search);
          break;
        case 'denied':
          refused = true;
          // The PC forgot this phone: stop presenting the old pairing.
          if (msg.reason === 'unknown') writePairing(null);
          if (this.typed) {
            this.typed = null;
            this.codeRefused = true;
          }
          this.status = 'unpaired';
          break;
        case 'welcome': {
          this.welcomed = true;
          this.status = 'online';
          this.config = msg.config;
          const topics = [...this.refs.keys()];
          if (topics.length) this.send({ t: 'sub', topics });
          break;
        }
        case 'config':
          this.config = msg.config;
          break;
        case 'data':
          if (msg.topic === 'agent.config') this.config = msg.payload;
          else this.data[msg.topic] = msg.payload;
          break;
        case 'result': {
          const done = this.pending.get(msg.id);
          if (done) {
            this.pending.delete(msg.id);
            done({ ok: msg.ok, error: msg.error });
          }
          break;
        }
      }
    };

    ws.onclose = () => {
      if (this.ws !== ws) return;
      this.ws = null;
      this.welcomed = false;
      if (!refused) this.status = 'offline';
      for (const done of this.pending.values()) done({ ok: false, error: 'offline' });
      this.pending.clear();
      // An unpaired phone keeps knocking, slowly: it gets in as soon as it is
      // paired, and a just-unpaired one retries once without its old pairing.
      this.retry = setTimeout(() => this.open(), refused ? 4000 : this.retryMs);
      this.retryMs = Math.min(this.retryMs * 2, 4000);
    };

    ws.onerror = () => ws.close();
  }

  private send(msg: unknown): boolean {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
      return true;
    }
    return false;
  }

  /** Subscribe to a topic; returns the unsubscribe function (use in $effect). */
  subscribe(topic: string): () => void {
    const n = this.refs.get(topic) ?? 0;
    this.refs.set(topic, n + 1);
    // Before the welcome the agent is not listening for this yet; everything
    // in `refs` is sent once it arrives.
    if (n === 0 && this.welcomed) this.send({ t: 'sub', topics: [topic] });
    return () => {
      const left = (this.refs.get(topic) ?? 1) - 1;
      if (left <= 0) {
        this.refs.delete(topic);
        if (this.welcomed) this.send({ t: 'unsub', topics: [topic] });
      } else {
        this.refs.set(topic, left);
      }
    };
  }

  private request(msg: Record<string, unknown>): Promise<Result> {
    const id = this.nextId++;
    return new Promise((resolve) => {
      if (!this.welcomed || !this.send({ ...msg, id })) return resolve({ ok: false, error: 'offline' });
      this.pending.set(id, resolve);
      setTimeout(() => {
        if (this.pending.delete(id)) resolve({ ok: false, error: 'timeout' });
      }, 5000);
    });
  }

  /** Built-in, parameter-safe actions (media, volume…). */
  action(action: Action): Promise<Result> {
    return this.request({ t: 'action', action });
  }

  /** Press a deck button; the agent looks the action up in its own config. */
  press(widget: string, button: number): Promise<Result> {
    return this.request({ t: 'press', widget, button });
  }
}

export const conn = new Connection();

export function buzz(ms = 12) {
  try {
    navigator.vibrate?.(ms);
  } catch {
    /* not supported */
  }
}
