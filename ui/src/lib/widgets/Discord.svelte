<script lang="ts">
  import { t } from '../strings';
  import { MicOff, Volume2 } from '@lucide/svelte';
  import { conn } from '../conn.svelte';
  import type { DiscordMember, DiscordServer, WidgetDef } from '../types';

  let { def }: { def: WidgetDef } = $props();

  $effect(() => conn.subscribe('discord.servers'));

  const servers = $derived(conn.data['discord.servers'] as DiscordServer[] | undefined);
  const title = $derived((def.config?.title as string) || 'Discord');
  const configured = $derived(((def.config?.servers as string[] | undefined) ?? []).some((id) => id.trim()));

  // Friends are matched by the name Discord shows, ignoring case.
  const friends = $derived(new Set(((def.config?.friends as string[] | undefined) ?? []).map((n) => n.trim().toLowerCase()).filter(Boolean)));
  const onlyFriends = $derived(def.config?.onlyFriends === true && friends.size > 0);
  const isFriend = (m: DiscordMember) => friends.has(m.name.toLowerCase());

  function listed(server: DiscordServer) {
    const members = onlyFriends ? server.members.filter(isFriend) : server.members;
    // Friends first, then people in voice, then the rest by name.
    return [...members].sort(
      (a, b) => Number(isFriend(b)) - Number(isFriend(a)) || Number(!!b.voice) - Number(!!a.voice) || a.name.localeCompare(b.name),
    );
  }

  const problems: Record<string, string> = {
    disabled: 'Server nemá zapnutý widget',
    unknown: 'Server s tímto ID neexistuje',
    offline: 'Discord není dostupný',
  };

  // A friend came online: let the hand on the desk know.
  let known: Set<string> | undefined;
  $effect(() => {
    if (!servers) return;
    const now = new Set(servers.flatMap((s) => s.members.filter(isFriend).map((m) => `${s.id}/${m.name.toLowerCase()}`)));
    if (known && def.config?.notify !== false) {
      for (const who of now) {
        if (!known.has(who)) {
          try {
            navigator.vibrate?.([80, 60, 80]);
          } catch {
            /* not supported */
          }
          break;
        }
      }
    }
    known = now;
  });
</script>

<div class="card discord">
  <div class="label">{title}</div>
  <div class="list">
    {#if !configured}
      <div class="quiet">{t('V editoru přidej ID serveru')}</div>
    {:else if !servers}
      <div class="quiet">{t('Zjišťuji…')}</div>
    {:else}
      {#each servers as server (server.id)}
        <div class="server">
          <span class="name">{server.name || server.id}</span>
          {#if !server.problem}<span class="count">{t('{n} online', { n: server.online })}</span>{/if}
        </div>
        {#if server.problem}
          <div class="quiet">{t(problems[server.problem])}</div>
        {:else}
          {#each listed(server) as m (m.name)}
            <div class="member {m.status}" class:friend={isFriend(m)}>
              <span class="face">
                {#if m.avatar}<img src="/discord/avatar/{m.avatar}" alt="" loading="lazy" />{/if}
                <i></i>
              </span>
              <span class="who">
                <span class="nick">{m.name}</span>
                {#if m.activity}<span class="doing">{m.activity}</span>{/if}
              </span>
              {#if m.voice}
                <span class="voice">
                  {#if m.muted}<MicOff size="2.8vmin" strokeWidth={2.4} />{:else}<Volume2 size="2.8vmin" strokeWidth={2.4} />{/if}
                  {m.voice}
                </span>
              {/if}
            </div>
          {:else}
            <div class="quiet">{onlyFriends ? t('Nikdo z přátel není online') : t('Nikdo není online')}</div>
          {/each}
        {/if}
      {/each}
    {/if}
  </div>
</div>

<style>
  .discord {
    display: flex;
    flex-direction: column;
  }
  .list {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    margin-top: 0.6vmin;
    scrollbar-width: none;
  }
  .list::-webkit-scrollbar {
    display: none;
  }
  .server {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-top: 1.4vmin;
    padding-bottom: 0.8vmin;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .name {
    font-weight: 700;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .count {
    flex: none;
    margin-left: 1.6vmin;
    font-size: 2.6vmin;
    font-weight: 700;
    color: var(--ok);
  }
  .member {
    --tone: var(--ok);
    display: flex;
    align-items: center;
    height: 7.4vmin;
    white-space: nowrap;
  }
  .idle {
    --tone: var(--warn);
  }
  .dnd {
    --tone: var(--bad);
  }
  .face {
    position: relative;
    width: 5.4vmin;
    height: 5.4vmin;
    flex: none;
    margin-right: 1.8vmin;
    border-radius: 50%;
    background: var(--card-hi);
  }
  .face img {
    width: 100%;
    height: 100%;
    border-radius: 50%;
    object-fit: cover;
    display: block;
  }
  .face i {
    position: absolute;
    right: -0.4vmin;
    bottom: -0.4vmin;
    width: 2vmin;
    height: 2vmin;
    border-radius: 50%;
    background: var(--tone);
    border: 0.4vmin solid var(--card);
  }
  .who {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    line-height: 1.2;
  }
  .nick {
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 3.1vmin;
    font-weight: 600;
  }
  .friend .nick {
    color: var(--accent);
  }
  .doing {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--mute);
    font-size: 2.4vmin;
  }
  .voice {
    display: inline-flex;
    align-items: center;
    flex: none;
    margin-left: auto;
    padding: 0 1.4vmin 0 1.6vmin;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--dim);
    font-size: 2.5vmin;
    font-weight: 600;
  }
  .voice :global(svg) {
    flex: none;
    margin-right: 0.8vmin;
  }
  .quiet {
    color: var(--mute);
    padding: 1.2vmin 0;
  }
</style>
