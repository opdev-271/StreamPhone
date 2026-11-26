import type { Action, Config } from './types';

type Result = { ok: boolean; error?: string };

/**
 * One WebSocket to the agent. Widgets subscribe to topics (ref-counted), the
 * agent only computes topics somebody is subscribed to.
 */
class Connection {
  status = $state<'connecting' | 'online' | 'offline'>('connecting');
  config = $state<Config | null>(null);
  data = $state<Record<string, any>>({});

  private ws: WebSocket | null = null;
  // Always listening for config changes made on the PC.
  private refs = new Map<string, number>([['agent.config', 1]]);
  private pending = new Map<number, (r: Result) => void>();
  private nextId = 1;
  private retryMs = 500;
  private started = false;

  connect() {
    if (this.started) return;
    this.started = true;
    this.open();
  }

  private open() {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    const ws = new WebSocket(`${proto}://${location.host}/ws`);
    this.ws = ws;

    ws.onopen = () => {
      this.status = 'online';
      this.retryMs = 500;
      const topics = [...this.refs.keys()];
      if (topics.length) this.send({ t: 'sub', topics });
    };

    ws.onmessage = (ev) => {
      let msg: any;
      try {
        msg = JSON.parse(ev.data);
      } catch {
        return;
      }
      switch (msg.t) {
        case 'welcome':
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
      this.status = 'offline';
      for (const done of this.pending.values()) done({ ok: false, error: 'offline' });
      this.pending.clear();
      setTimeout(() => this.open(), this.retryMs);
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
    if (n === 0) this.send({ t: 'sub', topics: [topic] });
    return () => {
      const left = (this.refs.get(topic) ?? 1) - 1;
      if (left <= 0) {
        this.refs.delete(topic);
        this.send({ t: 'unsub', topics: [topic] });
      } else {
        this.refs.set(topic, left);
      }
    };
  }

  private request(msg: Record<string, unknown>): Promise<Result> {
    const id = this.nextId++;
    return new Promise((resolve) => {
      if (!this.send({ ...msg, id })) return resolve({ ok: false, error: 'offline' });
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
