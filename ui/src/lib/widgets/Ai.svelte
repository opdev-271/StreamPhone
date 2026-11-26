<script lang="ts">
  import { Check, X } from '@lucide/svelte';
  import { buzz, conn } from '../conn.svelte';
  import type { AiSessions, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  $effect(() => conn.subscribe('ai.sessions'));

  const data = $derived(conn.data['ai.sessions'] as AiSessions | undefined);
  const sessions = $derived(data?.sessions ?? []);
  const canAnswer = $derived(def.config?.approve !== false);
  const title = $derived((def.config?.title as string) || 'AI relace');

  const tools: Record<string, string> = {
    claude: 'Claude',
    codex: 'Codex',
    gemini: 'Gemini',
    cursor: 'Cursor',
    copilot: 'Copilot',
    aider: 'Aider',
    opencode: 'opencode',
  };
  const toolName = (source: string) => tools[source] ?? source;

  const labels = { idle: 'Připraveno', working: 'Pracuje', waiting: 'Čeká na tebe', done: 'Hotovo' };

  // "před 3 min": refreshed twice a minute.
  let now = $state(Date.now());
  $effect(() => {
    const t = setInterval(() => (now = Date.now()), 30000);
    return () => clearInterval(t);
  });
  function ago(at: number) {
    const min = Math.floor(Math.max(0, now - at) / 60000);
    if (min < 1) return 'teď';
    if (min < 60) return `${min} min`;
    return `${Math.floor(min / 60)} h`;
  }

  function vibrate(pattern: number[]) {
    try {
      navigator.vibrate?.(pattern);
    } catch {
      /* not supported */
    }
  }

  // Buzz when a session asks for permission (long) or finishes (two short).
  let seen: { asked: number; finished: number } | undefined;
  $effect(() => {
    if (!data) return;
    if (seen) {
      if (data.asked > seen.asked) vibrate([400, 150, 400]);
      else if (data.finished > seen.finished) vibrate([120, 100, 120]);
    }
    seen = { asked: data.asked, finished: data.finished };
  });

  let sent = $state<number | null>(null);

  function decide(request: number, allow: boolean) {
    buzz(allow ? 12 : 30);
    sent = request;
    conn.action({ do: 'ai.decide', request, allow });
  }
</script>

<div class="card ai">
  <div class="head">
    <span class="label">{title}</span>
    <span class="label">{sessions.length || ''}</span>
  </div>

  <div class="list">
    {#each sessions as s (s.id)}
      <div class="session {s.status}" class:asking={!!s.request}>
        <div class="line">
          <i class="state"></i>
          <span class="project">{s.project || 'Relace'}</span>
          <span class="tool-tag {s.source}">{toolName(s.source)}</span>
          <span class="status">{labels[s.status]}</span>
          <span class="when">{ago(s.updatedAt)}</span>
        </div>
        {#if s.title}
          <div class="title">{s.title}</div>
        {/if}

        {#if s.request}
          <div class="ask">
            <div class="tool">{s.request.tool}</div>
            {#if s.request.detail}
              <div class="what">{s.request.detail}</div>
            {/if}
            {#if canAnswer}
              <div class="buttons">
                <button class="deny" disabled={sent === s.request.id} onclick={() => decide(s.request!.id, false)}>
                  <X size="4vmin" strokeWidth={2.8} />Zamítnout
                </button>
                <button class="allow" disabled={sent === s.request.id} onclick={() => decide(s.request!.id, true)}>
                  <Check size="4vmin" strokeWidth={2.8} />Povolit
                </button>
              </div>
            {/if}
          </div>
        {:else if s.detail}
          <div class="detail">{s.detail}</div>
        {/if}
      </div>
    {:else}
      <div class="empty">Žádná relace neběží</div>
    {/each}
  </div>
</div>

<style>
  .ai {
    display: flex;
    flex-direction: column;
  }
  .head {
    display: flex;
    justify-content: space-between;
  }
  .list {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    margin-top: 1vmin;
    scrollbar-width: none;
  }
  .list::-webkit-scrollbar {
    display: none;
  }
  .session {
    --tone: var(--mute);
    padding: 1.6vmin 0;
    border-bottom: 1px solid var(--line);
  }
  .session:last-child {
    border-bottom: 0;
  }
  .working {
    --tone: var(--accent);
  }
  .waiting {
    --tone: var(--warn);
  }
  .done {
    --tone: var(--ok);
  }
  .line {
    display: flex;
    align-items: center;
    white-space: nowrap;
  }
  .state {
    width: 1.8vmin;
    height: 1.8vmin;
    flex: none;
    margin-right: 1.6vmin;
    border-radius: 50%;
    background: var(--tone);
  }
  .working .state,
  .waiting .state {
    animation: pulse 1.4s ease-in-out infinite;
  }
  .project {
    font-weight: 700;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tool-tag {
    flex: none;
    margin-left: 1.4vmin;
    padding: 0 1.3vmin;
    border-radius: 2vmin;
    background: var(--line);
    color: var(--dim);
    font-size: 2.3vmin;
    font-weight: 700;
    line-height: 1.7;
  }
  .tool-tag.claude {
    background: rgba(217, 119, 87, 0.22);
    color: #e8956f;
  }
  .tool-tag.codex {
    background: rgba(16, 163, 127, 0.22);
    color: #2fc9a0;
  }
  .tool-tag.gemini {
    background: rgba(66, 133, 244, 0.22);
    color: #7fb0ff;
  }
  .status {
    margin-left: 1.6vmin;
    font-size: 2.6vmin;
    font-weight: 700;
    color: var(--tone);
    flex: none;
  }
  .when {
    margin-left: auto;
    padding-left: 1.6vmin;
    font-size: 2.6vmin;
    color: var(--mute);
    flex: none;
  }
  .title,
  .detail {
    margin-left: 3.4vmin;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 3vmin;
  }
  .title {
    color: var(--dim);
  }
  .detail {
    color: var(--mute);
    font-family: ui-monospace, 'Cascadia Mono', 'Roboto Mono', Consolas, monospace;
    font-size: 2.6vmin;
    margin-top: 0.3vmin;
  }
  .ask {
    margin-top: 1.4vmin;
    padding: 2vmin 2.2vmin;
    border-radius: 2.4vmin;
    background: var(--line);
  }
  .tool {
    font-weight: 700;
  }
  .what {
    margin-top: 0.4vmin;
    color: var(--dim);
    font-family: ui-monospace, 'Cascadia Mono', 'Roboto Mono', Consolas, monospace;
    font-size: 2.6vmin;
    line-height: 1.35;
    /* A command should be readable before it is allowed: up to three lines. */
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    word-break: break-all;
  }
  .buttons {
    display: flex;
    margin-top: 1.8vmin;
  }
  .buttons button {
    flex: 1;
    height: 8.4vmin;
    border-radius: 2vmin;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    color: #fff;
    transition: transform 0.4s var(--spring), opacity 0.2s;
  }
  .buttons button :global(svg) {
    margin-right: 1vmin;
  }
  .buttons button:active {
    transform: scale(0.95);
    transition: transform 0.07s;
  }
  .buttons button:disabled {
    opacity: 0.4;
  }
  .deny {
    background: var(--bad);
    margin-right: 1.6vmin;
  }
  .allow {
    background: var(--ok);
  }
  .empty {
    color: var(--mute);
    padding-top: 1.4vmin;
  }
  @keyframes pulse {
    50% {
      opacity: 0.3;
    }
  }
</style>
