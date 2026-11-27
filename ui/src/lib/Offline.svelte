<script lang="ts">
  import { t } from './strings';
  import { conn } from './conn.svelte';

  let now = $state(new Date());
  $effect(() => {
    const t = setInterval(() => (now = new Date()), 1000);
    return () => clearInterval(t);
  });
  // Always 24-hour: a 12-hour locale's "PM" does not fit the screen.
  const two = (n: number) => String(n).padStart(2, '0');
  const time = $derived(`${two(now.getHours())}:${two(now.getMinutes())}`);
</script>

<!-- Shown while the PC is unreachable: the phone still works as a clock. -->
<div class="offline">
  <div class="time">{time}</div>
  {#if conn.appLink}
    <!-- The pairing QR was read by the phone's camera and opened a browser. -->
    <div class="how">{t('Máš v telefonu aplikaci StreamPhone? Spáruj telefon v ní.')}</div>
    <div class="choice">
      <a class="go" href={conn.appLink}>{t('Otevřít v aplikaci')}</a>
      <button onclick={() => conn.useBrowser()}>{t('Pokračovat v prohlížeči')}</button>
    </div>
  {:else if conn.status === 'unpaired'}
    <div class="hint"><span class="pulse stop"></span>{t('Telefon není spárovaný')}</div>
    <div class="how">{t('Na počítači otevři editor StreamPhone, záložku Připojení, a naskenuj QR kód.')}</div>
  {:else}
    <div class="hint"><span class="pulse"></span>{t('Čekám na počítač…')}</div>
  {/if}
</div>

<style>
  .offline {
    position: fixed;
    top: 0;
    right: 0;
    bottom: 0;
    left: 0;
    background: var(--bg);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    z-index: 10;
  }
  .time {
    font-size: 34vmin;
    font-weight: 500;
    line-height: 1;
    letter-spacing: -0.045em;
  }
  .hint {
    margin-top: 5vmin;
    color: var(--mute);
    display: flex;
    align-items: center;
  }
  .pulse {
    width: 1.6vmin;
    height: 1.6vmin;
    border-radius: 50%;
    background: var(--warn);
    margin-right: 1.6vmin;
    animation: pulse 1.6s ease-in-out infinite;
  }
  .pulse.stop {
    background: var(--bad);
    animation: none;
  }
  .how {
    margin-top: 2vmin;
    max-width: 120vmin;
    padding: 0 6vmin;
    text-align: center;
    color: var(--dim);
    font-size: 3.2vmin;
  }
  .choice {
    margin-top: 4vmin;
    display: flex;
    align-items: center;
  }
  .choice a,
  .choice button {
    margin: 0 1.4vmin;
    padding: 2.6vmin 4.4vmin;
    border-radius: 3vmin;
    background: var(--line);
    color: var(--text);
    font-size: 3.4vmin;
    font-weight: 600;
    text-decoration: none;
  }
  .choice .go {
    background: var(--accent);
    color: #fff;
  }
  @keyframes pulse {
    50% {
      opacity: 0.25;
    }
  }
</style>
